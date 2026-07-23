use std::fs::File;
use std::io::{Read, Take};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::Path;

use rustix::fs::{self, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::state::{STATE_ROOT, StatePath};
use crate::state_store::{
    MAX_STATE_DOCUMENT_BYTES, StateRead, StateStoreError, StateStoreErrorKind,
};

const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const FILE_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// Read-only descriptor-relative access to one trusted SmolRunner state root.
#[derive(Debug)]
pub struct LinuxStateRoot {
    root: OwnedFd,
}

impl LinuxStateRoot {
    /// Open the canonical system state root without following a final symlink.
    ///
    /// # Errors
    ///
    /// Returns a bounded store error when the root is missing, inaccessible, symlinked, or not a
    /// directory.
    pub fn open_default() -> Result<Self, StateStoreError> {
        Self::open(STATE_ROOT)
    }

    /// Open one trusted state root for descriptor-relative reads.
    ///
    /// This constructor is public so integration tests and explicitly configured hosts can use a
    /// temporary or relocated root. Callers remain responsible for choosing the trusted root path.
    ///
    /// # Errors
    ///
    /// Returns a bounded store error when the root is missing, inaccessible, symlinked, or not a
    /// directory.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StateStoreError> {
        let root =
            fs::open(path.as_ref(), DIRECTORY_FLAGS, Mode::empty()).map_err(map_root_open_error)?;
        verify_directory(&root, "state root")?;
        Ok(Self { root })
    }

    /// Read one canonical state path without following symlinks in any component.
    ///
    /// # Errors
    ///
    /// Returns `UnsafeFilesystem` for symlinks, non-directory parents, or non-regular final files;
    /// `CorruptState` for oversized files; and `Io` for other bounded read failures.
    pub fn read(&self, path: &StatePath) -> Result<StateRead, StateStoreError> {
        let Some((file_name, parents)) = path.components().split_last() else {
            return Err(StateStoreError::public(
                StateStoreErrorKind::CorruptState,
                "state path contains no file component",
            ));
        };

        let mut current = None::<OwnedFd>;
        for component in parents {
            let dirfd = active_directory(&self.root, current.as_ref());
            current = match fs::openat(dirfd, component.as_str(), DIRECTORY_FLAGS, Mode::empty()) {
                Ok(directory) => {
                    verify_directory(&directory, "state path parent")?;
                    Some(directory)
                }
                Err(Errno::NOENT) => return Ok(StateRead::Missing),
                Err(error) => return Err(map_component_open_error(error)),
            };
        }

        let dirfd = active_directory(&self.root, current.as_ref());
        let file = match fs::openat(dirfd, file_name.as_str(), FILE_FLAGS, Mode::empty()) {
            Ok(file) => file,
            Err(Errno::NOENT) => return Ok(StateRead::Missing),
            Err(error) => return Err(map_component_open_error(error)),
        };
        verify_regular_file(&file)?;
        read_bounded(file)
    }
}

fn active_directory<'a>(root: &'a OwnedFd, current: Option<&'a OwnedFd>) -> BorrowedFd<'a> {
    match current {
        Some(directory) => directory.as_fd(),
        None => root.as_fd(),
    }
}

fn verify_directory(fd: &OwnedFd, subject: &str) -> Result<(), StateStoreError> {
    let stat = fs::fstat(fd).map_err(|_| {
        StateStoreError::public(
            StateStoreErrorKind::Io,
            format!("could not inspect {subject}"),
        )
    })?;
    if FileType::from_raw_mode(stat.st_mode).is_dir() {
        Ok(())
    } else {
        Err(StateStoreError::public(
            StateStoreErrorKind::UnsafeFilesystem,
            format!("{subject} is not a directory"),
        ))
    }
}

fn verify_regular_file(fd: &OwnedFd) -> Result<(), StateStoreError> {
    let stat = fs::fstat(fd).map_err(|_| {
        StateStoreError::public(StateStoreErrorKind::Io, "could not inspect state file")
    })?;
    if !FileType::from_raw_mode(stat.st_mode).is_file() {
        return Err(StateStoreError::public(
            StateStoreErrorKind::UnsafeFilesystem,
            "state path does not identify a regular file",
        ));
    }
    if stat.st_size < 0 || stat.st_size as u64 > MAX_STATE_DOCUMENT_BYTES as u64 {
        return Err(StateStoreError::public(
            StateStoreErrorKind::CorruptState,
            "state file exceeds the configured size limit",
        ));
    }
    Ok(())
}

fn read_bounded(fd: OwnedFd) -> Result<StateRead, StateStoreError> {
    let file = File::from(fd);
    let mut reader: Take<File> = file.take((MAX_STATE_DOCUMENT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|_| {
        StateStoreError::public(StateStoreErrorKind::Io, "could not read state file")
    })?;
    if bytes.len() > MAX_STATE_DOCUMENT_BYTES {
        return Err(StateStoreError::public(
            StateStoreErrorKind::CorruptState,
            "state file exceeds the configured size limit",
        ));
    }
    Ok(StateRead::Present(bytes))
}

fn map_root_open_error(error: Errno) -> StateStoreError {
    match error {
        Errno::LOOP | Errno::NOTDIR => StateStoreError::public(
            StateStoreErrorKind::UnsafeFilesystem,
            "state root is symlinked or not a directory",
        ),
        Errno::NOENT => {
            StateStoreError::public(StateStoreErrorKind::Io, "state root does not exist")
        }
        _ => StateStoreError::public(StateStoreErrorKind::Io, "could not open state root"),
    }
}

fn map_component_open_error(error: Errno) -> StateStoreError {
    match error {
        Errno::LOOP | Errno::NOTDIR => StateStoreError::public(
            StateStoreErrorKind::UnsafeFilesystem,
            "state path contains a symlink or non-directory parent",
        ),
        _ => StateStoreError::public(StateStoreErrorKind::Io, "could not open state path"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::state::{InstallationId, StateLayout};
    use crate::state_store::{MAX_STATE_DOCUMENT_BYTES, StateRead, StateStoreErrorKind};

    use super::LinuxStateRoot;

    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TempTree {
        path: PathBuf,
    }

    impl TempTree {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "smolrunner-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create isolated temporary root");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn installation_id() -> InstallationId {
        InstallationId::parse("0123456789abcdef").expect("installation ID")
    }

    fn create_project_parent(root: &Path) -> PathBuf {
        let installation = root.join("installations").join(installation_id().as_str());
        fs::create_dir_all(&installation).expect("create project parent");
        installation
    }

    #[test]
    fn reads_regular_file_through_canonical_components() {
        let root = TempTree::new("regular-read");
        let parent = create_project_parent(root.path());
        fs::write(parent.join("project.json"), b"{\"schema_version\":1}\n")
            .expect("write project state");

        let reader = LinuxStateRoot::open(root.path()).expect("open state root");
        assert_eq!(
            reader
                .read(&StateLayout::project_document(&installation_id()))
                .expect("read state"),
            StateRead::Present(b"{\"schema_version\":1}\n".to_vec())
        );
    }

    #[test]
    fn missing_parent_or_file_is_reported_as_missing() {
        let root = TempTree::new("missing");
        let reader = LinuxStateRoot::open(root.path()).expect("open state root");
        assert_eq!(
            reader
                .read(&StateLayout::project_document(&installation_id()))
                .expect("read missing state"),
            StateRead::Missing
        );

        create_project_parent(root.path());
        assert_eq!(
            reader
                .read(&StateLayout::project_document(&installation_id()))
                .expect("read missing file"),
            StateRead::Missing
        );
    }

    #[test]
    fn symlinked_parent_is_rejected() {
        let root = TempTree::new("parent-link");
        let outside = TempTree::new("parent-link-outside");
        symlink(outside.path(), root.path().join("installations")).expect("create parent symlink");

        let reader = LinuxStateRoot::open(root.path()).expect("open state root");
        let error = reader
            .read(&StateLayout::project_document(&installation_id()))
            .expect_err("parent symlink must fail");
        assert_eq!(error.kind(), StateStoreErrorKind::UnsafeFilesystem);
    }

    #[test]
    fn symlinked_final_file_is_rejected() {
        let root = TempTree::new("file-link");
        let outside = TempTree::new("file-link-outside");
        let outside_file = outside.path().join("project.json");
        fs::write(&outside_file, b"foreign").expect("write foreign file");
        let parent = create_project_parent(root.path());
        symlink(&outside_file, parent.join("project.json")).expect("create file symlink");

        let reader = LinuxStateRoot::open(root.path()).expect("open state root");
        let error = reader
            .read(&StateLayout::project_document(&installation_id()))
            .expect_err("file symlink must fail");
        assert_eq!(error.kind(), StateStoreErrorKind::UnsafeFilesystem);
    }

    #[test]
    fn symlinked_root_is_rejected() {
        let actual = TempTree::new("actual-root");
        let link_parent = TempTree::new("root-link-parent");
        let link = link_parent.path().join("state-link");
        symlink(actual.path(), &link).expect("create root symlink");

        let error = LinuxStateRoot::open(&link).expect_err("root symlink must fail");
        assert_eq!(error.kind(), StateStoreErrorKind::UnsafeFilesystem);
    }

    #[test]
    fn non_regular_and_oversized_files_are_rejected() {
        let root = TempTree::new("invalid-files");
        let parent = create_project_parent(root.path());
        fs::create_dir(parent.join("project.json")).expect("create directory at file path");
        let reader = LinuxStateRoot::open(root.path()).expect("open state root");
        let error = reader
            .read(&StateLayout::project_document(&installation_id()))
            .expect_err("directory at file path must fail");
        assert_eq!(error.kind(), StateStoreErrorKind::UnsafeFilesystem);

        fs::remove_dir(parent.join("project.json")).expect("remove directory");
        fs::write(
            parent.join("project.json"),
            vec![0_u8; MAX_STATE_DOCUMENT_BYTES + 1],
        )
        .expect("write oversized state");
        let error = reader
            .read(&StateLayout::project_document(&installation_id()))
            .expect_err("oversized state must fail");
        assert_eq!(error.kind(), StateStoreErrorKind::CorruptState);
    }
}
