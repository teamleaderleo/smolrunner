use std::fs::File;
use std::io::{Read, Take};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::Path;

use rustix::fs::{self, Dir, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::linux_installation_catalog_lock::InstallationCatalogLock;
use crate::ownership::ProjectIdentity;
use crate::state::{InstallationId, STATE_ROOT};
use crate::state_document::{ProjectStateDocument, StateDocument, decode_state_document};
use crate::state_store::{MAX_STATE_DOCUMENT_BYTES, StateStoreError, StateStoreErrorKind};

const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const FILE_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const MAX_INSTALLATIONS: usize = 1_024;
const PROJECT_FILE: &str = "project.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallationLookup {
    Missing,
    Found(InstallationId),
}

impl InstallationLookup {
    #[must_use]
    pub fn installation_id(&self) -> Option<&InstallationId> {
        match self {
            Self::Missing => None,
            Self::Found(id) => Some(id),
        }
    }
}

/// Find the unique installation for a project beneath the canonical state root.
///
/// # Errors
///
/// Returns a bounded error for unsafe, malformed, incomplete, or duplicate catalog state.
pub fn find_default_installation(
    project: &ProjectIdentity,
) -> Result<InstallationLookup, StateStoreError> {
    find_installation(STATE_ROOT, project)
}

/// Find the unique installation for a project beneath a trusted state root.
///
/// This lookup is read-only. Every catalog entry and project document must validate completely.
///
/// # Errors
///
/// Returns a bounded error for unsafe, malformed, incomplete, or duplicate catalog state.
pub fn find_installation(
    root_path: impl AsRef<Path>,
    project: &ProjectIdentity,
) -> Result<InstallationLookup, StateStoreError> {
    let root = open_root(root_path.as_ref())?;
    let root_stat = inspect_directory(root.as_fd(), "state root", None)?;
    find_in_open_root(
        root.as_fd(),
        (root_stat.st_uid, root_stat.st_gid),
        project,
    )
}

/// Find the unique installation for a project beneath the exact root held by a catalog lock.
///
/// This form is intended for create-or-load operations that must keep one root descriptor and one
/// exclusive catalog lock across lookup and publication.
///
/// # Errors
///
/// Returns a bounded error for unsafe, malformed, incomplete, or duplicate catalog state.
pub fn find_locked_installation(
    catalog_lock: &InstallationCatalogLock,
    project: &ProjectIdentity,
) -> Result<InstallationLookup, StateStoreError> {
    inspect_directory(
        catalog_lock.root(),
        "state root",
        Some(catalog_lock.owner()),
    )?;
    find_in_open_root(catalog_lock.root(), catalog_lock.owner(), project)
}

fn find_in_open_root(
    root: BorrowedFd<'_>,
    owner: (u32, u32),
    project: &ProjectIdentity,
) -> Result<InstallationLookup, StateStoreError> {
    let Some(installations) = open_installations(root)? else {
        return Ok(InstallationLookup::Missing);
    };
    inspect_directory(
        installations.as_fd(),
        "installations directory",
        Some(owner),
    )?;

    let mut entries = Dir::read_from(&installations)
        .map_err(|_| io_error("could not enumerate the installation catalog"))?;
    let mut count = 0_usize;
    let mut found = None;
    for entry in &mut entries {
        let entry = entry.map_err(|_| io_error("could not read a catalog entry"))?;
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        count += 1;
        if count > MAX_INSTALLATIONS {
            return Err(corrupt_error("installation catalog is too large"));
        }

        let id = parse_installation_id(name)?;
        let directory = open_installation(&installations, &id)?;
        inspect_directory(directory.as_fd(), "installation directory", Some(owner))?;
        let document = read_project(&directory, owner)?;
        if document.installation_id() != &id {
            return Err(corrupt_error("project state ID differs from its directory"));
        }
        if document.project() == project && found.replace(id).is_some() {
            return Err(corrupt_error("multiple installations claim the project"));
        }
    }

    Ok(match found {
        Some(id) => InstallationLookup::Found(id),
        None => InstallationLookup::Missing,
    })
}

fn parse_installation_id(name: &[u8]) -> Result<InstallationId, StateStoreError> {
    let name =
        std::str::from_utf8(name).map_err(|_| corrupt_error("catalog entry name is not UTF-8"))?;
    InstallationId::parse(name)
        .map_err(|_| corrupt_error("catalog entry is not a valid installation ID"))
}

fn open_root(path: &Path) -> Result<OwnedFd, StateStoreError> {
    fs::open(path, DIRECTORY_FLAGS, Mode::empty()).map_err(|error| match error {
        Errno::LOOP | Errno::NOTDIR => unsafe_error("state root is symlinked or invalid"),
        Errno::NOENT => io_error("state root does not exist"),
        _ => io_error("could not open state root"),
    })
}

fn open_installations(root: BorrowedFd<'_>) -> Result<Option<OwnedFd>, StateStoreError> {
    match fs::openat(root, "installations", DIRECTORY_FLAGS, Mode::empty()) {
        Ok(directory) => Ok(Some(directory)),
        Err(Errno::NOENT) => Ok(None),
        Err(Errno::LOOP | Errno::NOTDIR) => {
            Err(unsafe_error("installations path is symlinked or invalid"))
        }
        Err(_) => Err(io_error("could not open installations directory")),
    }
}

fn open_installation(
    installations: &OwnedFd,
    id: &InstallationId,
) -> Result<OwnedFd, StateStoreError> {
    fs::openat(installations, id.as_str(), DIRECTORY_FLAGS, Mode::empty()).map_err(|error| {
        match error {
            Errno::LOOP | Errno::NOTDIR => unsafe_error("catalog entry is symlinked or invalid"),
            Errno::NOENT => corrupt_error("catalog changed during inspection"),
            _ => io_error("could not open catalog entry"),
        }
    })
}

fn inspect_directory(
    directory: BorrowedFd<'_>,
    subject: &str,
    expected_owner: Option<(u32, u32)>,
) -> Result<rustix::fs::Stat, StateStoreError> {
    let stat =
        fs::fstat(directory).map_err(|_| io_error(format!("could not inspect {subject}")))?;
    if !FileType::from_raw_mode(stat.st_mode).is_dir() {
        return Err(unsafe_error(format!("{subject} is not a directory")));
    }
    if stat.st_mode & 0o7777 != 0o750 {
        return Err(unsafe_error(format!("{subject} does not have mode 0750")));
    }
    if expected_owner.is_some_and(|owner| owner != (stat.st_uid, stat.st_gid)) {
        return Err(unsafe_error(format!(
            "{subject} has the wrong owner or group"
        )));
    }
    Ok(stat)
}

fn read_project(
    installation: &OwnedFd,
    owner: (u32, u32),
) -> Result<ProjectStateDocument, StateStoreError> {
    let file =
        fs::openat(installation, PROJECT_FILE, FILE_FLAGS, Mode::empty()).map_err(|error| {
            match error {
                Errno::LOOP | Errno::NOTDIR => {
                    unsafe_error("project state is symlinked or invalid")
                }
                Errno::NOENT => corrupt_error("installation is missing project state"),
                _ => io_error("could not open project state"),
            }
        })?;
    inspect_project_file(&file, owner)?;
    let bytes = read_bounded(file)?;
    let input = std::str::from_utf8(&bytes)
        .map_err(|_| corrupt_error("project state is not valid UTF-8"))?;
    let document = decode_state_document(input)
        .map_err(|_| corrupt_error("project state document is invalid"))?;
    match document {
        StateDocument::Project(project) => Ok(project),
        StateDocument::Resource(_) => Err(corrupt_error("project path contains a resource record")),
    }
}

fn inspect_project_file(file: &OwnedFd, owner: (u32, u32)) -> Result<(), StateStoreError> {
    let stat = fs::fstat(file).map_err(|_| io_error("could not inspect project state"))?;
    if !FileType::from_raw_mode(stat.st_mode).is_file() {
        return Err(unsafe_error("project state is not a regular file"));
    }
    if stat.st_nlink != 1 {
        return Err(unsafe_error("project state has multiple hard links"));
    }
    if stat.st_mode & 0o7777 != 0o600 {
        return Err(unsafe_error("project state does not have mode 0600"));
    }
    if owner != (stat.st_uid, stat.st_gid) {
        return Err(unsafe_error("project state has the wrong owner or group"));
    }
    if stat.st_size < 0 || stat.st_size as u64 > MAX_STATE_DOCUMENT_BYTES as u64 {
        return Err(corrupt_error("project state exceeds the size limit"));
    }
    Ok(())
}

fn read_bounded(file: OwnedFd) -> Result<Vec<u8>, StateStoreError> {
    let file = File::from(file);
    let mut reader: Take<File> = file.take((MAX_STATE_DOCUMENT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    let read_result = reader.read_to_end(&mut bytes);
    read_result.map_err(|_| io_error("could not read project state"))?;
    if bytes.len() > MAX_STATE_DOCUMENT_BYTES {
        return Err(corrupt_error("project state exceeds the size limit"));
    }
    Ok(bytes)
}

fn io_error(message: impl Into<String>) -> StateStoreError {
    StateStoreError::public(StateStoreErrorKind::Io, message)
}

fn corrupt_error(message: impl Into<String>) -> StateStoreError {
    StateStoreError::public(StateStoreErrorKind::CorruptState, message)
}

fn unsafe_error(message: impl Into<String>) -> StateStoreError {
    StateStoreError::public(StateStoreErrorKind::UnsafeFilesystem, message)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::linux_installation_catalog_lock::lock_installation_catalog;
    use crate::linux_state::LinuxStateRoot;
    use crate::linux_state_prepare::prepare_installation;
    use crate::manifest::RunnerScope;
    use crate::ownership::ProjectIdentity;
    use crate::state::InstallationId;
    use crate::state_document::ProjectStateDocument;
    use crate::state_store::{StateRecord, StateStoreErrorKind};

    use super::{
        InstallationLookup, PROJECT_FILE, find_installation, find_locked_installation,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let name = format!(
                "smolrunner-catalog-{label}-{}-{sequence}",
                std::process::id()
            );
            let path = std::env::temp_dir().join(name);
            fs::create_dir(&path).expect("create root");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).expect("set root mode");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn project(repository: &str) -> ProjectIdentity {
        ProjectIdentity {
            repository: repository.to_owned(),
            runner_scope: RunnerScope::Repository,
            runner_user: "project-runner".to_owned(),
        }
    }

    fn id(value: &str) -> InstallationId {
        InstallationId::parse(value).expect("installation ID")
    }

    fn write_project(root: &Path, id: InstallationId, project: ProjectIdentity) {
        prepare_installation(root, &id).expect("prepare installation");
        let document = ProjectStateDocument::new(id, project).expect("project document");
        let record = StateRecord::project(document).expect("project record");
        let mut store = LinuxStateRoot::open(root).expect("open store");
        store.write_atomic(&record).expect("write project record");
    }

    #[test]
    fn missing_or_unmatched_project_returns_missing() {
        let root = TempRoot::new("missing");
        let expected = InstallationLookup::Missing;
        let actual = find_installation(root.path(), &project("example/project"));
        assert_eq!(actual.expect("empty lookup"), expected);

        write_project(
            root.path(),
            id("1111111111111111"),
            project("example/other"),
        );
        let actual = find_installation(root.path(), &project("example/project"));
        assert_eq!(actual.expect("unmatched lookup"), expected);
    }

    #[test]
    fn exact_project_returns_its_installation() {
        let root = TempRoot::new("found");
        let expected = id("2222222222222222");
        write_project(root.path(), expected.clone(), project("example/project"));
        let actual = find_installation(root.path(), &project("example/project"));
        assert_eq!(actual.expect("lookup"), InstallationLookup::Found(expected));
    }

    #[test]
    fn locked_lookup_uses_the_catalog_locks_open_root() {
        let root = TempRoot::new("locked");
        let expected = id("2323232323232323");
        let target = project("example/project");
        write_project(root.path(), expected.clone(), target.clone());
        let lock = lock_installation_catalog(root.path()).expect("lock catalog");
        assert_eq!(
            find_locked_installation(&lock, &target).expect("locked lookup"),
            InstallationLookup::Found(expected)
        );
    }

    #[test]
    fn duplicate_project_claims_fail_closed() {
        let root = TempRoot::new("duplicate");
        let target = project("example/project");
        write_project(root.path(), id("3333333333333333"), target.clone());
        write_project(root.path(), id("4444444444444444"), target.clone());
        let error = find_installation(root.path(), &target).expect_err("duplicate claim");
        assert_eq!(error.kind(), StateStoreErrorKind::CorruptState);
    }

    #[test]
    fn invalid_catalog_entry_fails_closed() {
        let root = TempRoot::new("invalid");
        let path = root.path().join("installations");
        fs::create_dir(&path).expect("create installations");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).expect("set mode");
        let invalid = path.join("INVALID");
        fs::create_dir(&invalid).expect("create invalid entry");
        fs::set_permissions(&invalid, fs::Permissions::from_mode(0o750)).expect("set mode");

        let error = find_installation(root.path(), &project("example/project"));
        assert_eq!(
            error.expect_err("invalid entry").kind(),
            StateStoreErrorKind::CorruptState
        );
    }

    #[test]
    fn broad_project_mode_is_rejected() {
        let root = TempRoot::new("broad");
        let installation = id("5555555555555555");
        write_project(
            root.path(),
            installation.clone(),
            project("example/project"),
        );
        let path = root
            .path()
            .join("installations")
            .join(installation.as_str())
            .join(PROJECT_FILE);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("broaden mode");

        let error = find_installation(root.path(), &project("example/project"));
        assert_eq!(
            error.expect_err("broad mode").kind(),
            StateStoreErrorKind::UnsafeFilesystem
        );
    }
}
