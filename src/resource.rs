use std::fmt;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::journal::ExecutionLane;
use crate::ownership::{ResourceEvidence, ResourceIdentity, ResourceKind};

pub const CANONICAL_RESOURCE_SCHEMA_VERSION: u8 = 1;

const MAX_NAME_LEN: usize = 100;
const MAX_PATH_LEN: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountPolicy {
    Login,
    Service,
    Locked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GithubScope {
    Repository { id: u64, slug: String },
    Organization { id: u64, login: String },
}

impl GithubScope {
    /// Build a canonical repository scope from a stable numeric repository ID and a current slug.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the ID is zero or the slug is not exact `OWNER/REPOSITORY`.
    pub fn repository(id: u64, slug: &str) -> Result<Self, CanonicalResourceError> {
        let slug = canonical_repository_slug(slug)?;
        if id == 0 {
            return Err(CanonicalResourceError::single(
                "repository scope ID must be greater than zero",
            ));
        }
        Ok(Self::Repository { id, slug })
    }

    /// Build a canonical organization scope from a stable numeric organization ID and login.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the ID is zero or the login is unsafe.
    pub fn organization(id: u64, login: &str) -> Result<Self, CanonicalResourceError> {
        if id == 0 {
            return Err(CanonicalResourceError::single(
                "organization scope ID must be greater than zero",
            ));
        }
        let login = canonical_github_name("organization login", login)?;
        Ok(Self::Organization { id, login })
    }

    #[must_use]
    pub fn locator_prefix(&self) -> String {
        match self {
            Self::Repository { id, .. } => format!("repository-id:{id}"),
            Self::Organization { id, .. } => format!("organization-id:{id}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EvidenceSurvival {
    pub host_restore: bool,
    pub repository_transfer: bool,
    pub runner_reregistration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourcePolicy {
    pub kind: ResourceKind,
    pub observation_lanes: Vec<ExecutionLane>,
    pub survival: EvidenceSurvival,
}

#[must_use]
pub fn policy(kind: ResourceKind) -> ResourcePolicy {
    let (observation_lanes, survival) = match kind {
        ResourceKind::LinuxUser => (
            vec![ExecutionLane::Root],
            EvidenceSurvival {
                host_restore: false,
                repository_transfer: true,
                runner_reregistration: true,
            },
        ),
        ResourceKind::Directory => (
            vec![ExecutionLane::Root],
            EvidenceSurvival {
                host_restore: true,
                repository_transfer: true,
                runner_reregistration: true,
            },
        ),
        ResourceKind::SystemdService => (
            vec![ExecutionLane::Root],
            EvidenceSurvival {
                host_restore: true,
                repository_transfer: true,
                runner_reregistration: true,
            },
        ),
        ResourceKind::RunnerInstallation => (
            vec![ExecutionLane::RunnerUser, ExecutionLane::Root],
            EvidenceSurvival {
                host_restore: true,
                repository_transfer: false,
                runner_reregistration: false,
            },
        ),
        ResourceKind::PodmanImage => (
            vec![ExecutionLane::RunnerUser],
            EvidenceSurvival {
                host_restore: false,
                repository_transfer: true,
                runner_reregistration: true,
            },
        ),
        ResourceKind::GithubRunnerRegistration => (
            vec![ExecutionLane::Github],
            EvidenceSurvival {
                host_restore: true,
                repository_transfer: false,
                runner_reregistration: false,
            },
        ),
    };

    ResourcePolicy {
        kind,
        observation_lanes,
        survival,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalResourceError {
    pub problems: Vec<String>,
}

impl CanonicalResourceError {
    fn single(problem: impl Into<String>) -> Self {
        Self {
            problems: vec![problem.into()],
        }
    }
}

impl fmt::Display for CanonicalResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "canonical resource validation failed")?;
        for problem in &self.problems {
            writeln!(formatter, "- {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CanonicalResourceError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinuxUserFingerprint {
    schema_version: u8,
    primary_gid: u32,
    home: String,
    shell: String,
    account_policy: AccountPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryFingerprint {
    schema_version: u8,
    owner_uid: u32,
    owner_gid: u32,
    mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SystemdServiceFingerprint {
    schema_version: u8,
    unit_file: String,
    content_digest: String,
    service_user: String,
    runner_installation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunnerInstallationFingerprint {
    schema_version: u8,
    server_url: String,
    scope: GithubScope,
    service_unit: String,
    runner_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PodmanImageFingerprint {
    schema_version: u8,
    build_input_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GithubRegistrationFingerprint {
    schema_version: u8,
    scope: GithubScope,
}

impl ResourceIdentity {
    /// Build canonical Linux-user identity from the exact account tuple.
    ///
    /// # Errors
    ///
    /// Returns a validation error for unsafe usernames, noncanonical paths, UID zero, or an
    /// account-policy mismatch.
    pub fn linux_user(
        username: &str,
        uid: u32,
        primary_gid: u32,
        home: &str,
        shell: &str,
        account_policy: AccountPolicy,
    ) -> Result<Self, CanonicalResourceError> {
        validate_linux_user(username)?;
        if uid == 0 {
            return Err(CanonicalResourceError::single(
                "runner Linux user must not have UID zero",
            ));
        }
        let home = canonical_absolute_path("Linux user home", home)?;
        let shell = canonical_absolute_path("Linux user shell", shell)?;
        if account_policy == AccountPolicy::Login && shell.ends_with("/nologin") {
            return Err(CanonicalResourceError::single(
                "login account policy cannot use a nologin shell",
            ));
        }

        let fingerprint = LinuxUserFingerprint {
            schema_version: CANONICAL_RESOURCE_SCHEMA_VERSION,
            primary_gid,
            home,
            shell,
            account_policy,
        };
        Ok(Self::new(
            ResourceKind::LinuxUser,
            username,
            ResourceEvidence::both(
                format!("uid:{uid}"),
                encode_fingerprint("linux-user", &fingerprint)?,
            ),
        ))
    }

    /// Build canonical managed-directory identity.
    ///
    /// The installation ID is required evidence, so an unmarked directory cannot be adopted only
    /// because its path and mode happen to match.
    ///
    /// # Errors
    ///
    /// Returns a validation error for path aliases, unsafe installation IDs, or invalid modes.
    pub fn directory(
        path: &str,
        installation_id: &str,
        owner_uid: u32,
        owner_gid: u32,
        mode: u32,
    ) -> Result<Self, CanonicalResourceError> {
        let path = canonical_absolute_path("directory", path)?;
        validate_installation_id(installation_id)?;
        validate_mode(mode)?;
        let fingerprint = DirectoryFingerprint {
            schema_version: CANONICAL_RESOURCE_SCHEMA_VERSION,
            owner_uid,
            owner_gid,
            mode,
        };
        Ok(Self::new(
            ResourceKind::Directory,
            path,
            ResourceEvidence::both(
                format!("installation:{installation_id}"),
                encode_fingerprint("directory", &fingerprint)?,
            ),
        ))
    }

    /// Build canonical systemd-service identity from an escaped unit and immutable unit content.
    ///
    /// # Errors
    ///
    /// Returns a validation error for unsafe unit names, paths, digests, users, or installation IDs.
    pub fn systemd_service(
        unit: &str,
        unit_file: &str,
        content_digest: &str,
        service_user: &str,
        runner_installation_id: &str,
    ) -> Result<Self, CanonicalResourceError> {
        validate_systemd_unit(unit)?;
        let unit_file = canonical_absolute_path("systemd unit file", unit_file)?;
        validate_sha256("systemd unit content digest", content_digest)?;
        validate_linux_user(service_user)?;
        validate_installation_id(runner_installation_id)?;
        let fingerprint = SystemdServiceFingerprint {
            schema_version: CANONICAL_RESOURCE_SCHEMA_VERSION,
            unit_file,
            content_digest: content_digest.to_owned(),
            service_user: service_user.to_owned(),
            runner_installation_id: runner_installation_id.to_owned(),
        };
        Ok(Self::new(
            ResourceKind::SystemdService,
            unit,
            ResourceEvidence::both(
                format!("runner-installation:{runner_installation_id}"),
                encode_fingerprint("systemd-service", &fingerprint)?,
            ),
        ))
    }

    /// Build canonical official-runner installation identity from local `.runner` evidence.
    ///
    /// # Errors
    ///
    /// Returns a validation error for path aliases, unsafe URLs, scope data, units, versions, or
    /// runner IDs.
    pub fn runner_installation(
        directory: &str,
        server_url: &str,
        scope: GithubScope,
        runner_id: Option<u64>,
        service_unit: &str,
        runner_version: &str,
    ) -> Result<Self, CanonicalResourceError> {
        let directory = canonical_absolute_path("runner installation", directory)?;
        validate_server_url(server_url)?;
        validate_scope(&scope)?;
        if runner_id == Some(0) {
            return Err(CanonicalResourceError::single(
                "GitHub runner ID must be greater than zero",
            ));
        }
        validate_systemd_unit(service_unit)?;
        validate_runner_version(runner_version)?;
        let fingerprint = RunnerInstallationFingerprint {
            schema_version: CANONICAL_RESOURCE_SCHEMA_VERSION,
            server_url: server_url.to_owned(),
            scope,
            service_unit: service_unit.to_owned(),
            runner_version: runner_version.to_owned(),
        };
        let fingerprint = encode_fingerprint("runner-installation", &fingerprint)?;
        let evidence = match runner_id {
            Some(id) => ResourceEvidence::both(format!("github-runner:{id}"), fingerprint),
            None => ResourceEvidence::fingerprint(fingerprint),
        };
        Ok(Self::new(
            ResourceKind::RunnerInstallation,
            directory,
            evidence,
        ))
    }

    /// Build canonical rootless-Podman image identity.
    ///
    /// # Errors
    ///
    /// Returns a validation error when the local reference is unsafe or immutable digests are absent.
    pub fn podman_image(
        local_reference: &str,
        image_digest: &str,
        build_input_digest: &str,
    ) -> Result<Self, CanonicalResourceError> {
        validate_local_image_reference(local_reference)?;
        validate_sha256("Podman image digest", image_digest)?;
        validate_sha256("Podman build-input digest", build_input_digest)?;
        let fingerprint = PodmanImageFingerprint {
            schema_version: CANONICAL_RESOURCE_SCHEMA_VERSION,
            build_input_digest: build_input_digest.to_owned(),
        };
        Ok(Self::new(
            ResourceKind::PodmanImage,
            local_reference,
            ResourceEvidence::both(
                format!("image:{image_digest}"),
                encode_fingerprint("podman-image", &fingerprint)?,
            ),
        ))
    }

    /// Build canonical GitHub runner-registration identity from authenticated API evidence.
    ///
    /// # Errors
    ///
    /// Returns a validation error for missing numeric IDs, unsafe scopes, or unsafe runner names.
    pub fn github_runner_registration(
        scope: GithubScope,
        runner_name: &str,
        runner_id: u64,
    ) -> Result<Self, CanonicalResourceError> {
        validate_scope(&scope)?;
        let runner_name = canonical_runner_name(runner_name)?;
        if runner_id == 0 {
            return Err(CanonicalResourceError::single(
                "GitHub runner ID must be greater than zero",
            ));
        }
        let locator = format!("{}/runner:{runner_name}", scope.locator_prefix());
        let fingerprint = GithubRegistrationFingerprint {
            schema_version: CANONICAL_RESOURCE_SCHEMA_VERSION,
            scope,
        };
        Ok(Self::new(
            ResourceKind::GithubRunnerRegistration,
            locator,
            ResourceEvidence::both(
                format!("github-runner:{runner_id}"),
                encode_fingerprint("github-registration", &fingerprint)?,
            ),
        ))
    }
}

/// Validate a desired resource identity against its kind-specific canonical format.
///
/// Desired identities must contain the minimum immutable evidence for their resource kind.
#[must_use]
pub fn validate_identity(identity: &ResourceIdentity) -> Vec<String> {
    validate_identity_with_requirement(identity, EvidenceRequirement::Complete)
}

/// Validate an observed or marker resource identity.
///
/// Missing evidence is allowed so ownership classification can report `unknown`; any evidence that
/// is present must still use the exact canonical format.
#[must_use]
pub(crate) fn validate_observed_identity(identity: &ResourceIdentity) -> Vec<String> {
    validate_identity_with_requirement(identity, EvidenceRequirement::Partial)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceRequirement {
    Complete,
    Partial,
}

impl EvidenceRequirement {
    const fn requires(self, present: bool) -> bool {
        self == Self::Complete || present
    }
}

fn validate_identity_with_requirement(
    identity: &ResourceIdentity,
    requirement: EvidenceRequirement,
) -> Vec<String> {
    let mut problems = Vec::new();
    match identity.kind {
        ResourceKind::LinuxUser => {
            validate_linux_user_identity(identity, requirement, &mut problems);
        }
        ResourceKind::Directory => {
            validate_directory_identity(identity, requirement, &mut problems);
        }
        ResourceKind::SystemdService => {
            validate_systemd_identity(identity, requirement, &mut problems);
        }
        ResourceKind::RunnerInstallation => {
            validate_runner_installation_identity(identity, requirement, &mut problems);
        }
        ResourceKind::PodmanImage => {
            validate_podman_image_identity(identity, requirement, &mut problems);
        }
        ResourceKind::GithubRunnerRegistration => {
            validate_github_registration_identity(identity, requirement, &mut problems);
        }
    }
    problems
}

fn validate_linux_user_identity(
    identity: &ResourceIdentity,
    requirement: EvidenceRequirement,
    problems: &mut Vec<String>,
) {
    collect(validate_linux_user(&identity.locator), problems);
    if requirement.requires(identity.evidence.external_id.is_some()) {
        collect(
            validate_prefixed_u64(
                "Linux user external ID",
                identity.evidence.external_id.as_deref(),
                "uid:",
                true,
            ),
            problems,
        );
    }
    if let Some(fingerprint) = decode_fingerprint::<LinuxUserFingerprint>(
        "linux-user",
        identity.evidence.fingerprint.as_deref(),
        requirement,
        problems,
    ) {
        collect(
            canonical_absolute_path("Linux user home", &fingerprint.home).map(|_| ()),
            problems,
        );
        collect(
            canonical_absolute_path("Linux user shell", &fingerprint.shell).map(|_| ()),
            problems,
        );
        if fingerprint.schema_version != CANONICAL_RESOURCE_SCHEMA_VERSION {
            problems.push("Linux user fingerprint version is not supported".to_owned());
        }
        if fingerprint.account_policy == AccountPolicy::Login
            && fingerprint.shell.ends_with("/nologin")
        {
            problems.push("login account policy cannot use a nologin shell".to_owned());
        }
    }
}

fn validate_directory_identity(
    identity: &ResourceIdentity,
    requirement: EvidenceRequirement,
    problems: &mut Vec<String>,
) {
    collect(
        canonical_absolute_path("directory", &identity.locator).map(|_| ()),
        problems,
    );
    if requirement.requires(identity.evidence.external_id.is_some()) {
        collect(
            validate_prefixed_token(
                "directory external ID",
                identity.evidence.external_id.as_deref(),
                "installation:",
            ),
            problems,
        );
    }
    if let Some(fingerprint) = decode_fingerprint::<DirectoryFingerprint>(
        "directory",
        identity.evidence.fingerprint.as_deref(),
        requirement,
        problems,
    ) {
        if fingerprint.schema_version != CANONICAL_RESOURCE_SCHEMA_VERSION {
            problems.push("directory fingerprint version is not supported".to_owned());
        }
        collect(validate_mode(fingerprint.mode), problems);
    }
}

fn validate_systemd_identity(
    identity: &ResourceIdentity,
    requirement: EvidenceRequirement,
    problems: &mut Vec<String>,
) {
    collect(validate_systemd_unit(&identity.locator), problems);
    let external = identity.evidence.external_id.as_deref();
    if requirement.requires(external.is_some()) {
        collect(
            validate_prefixed_token(
                "systemd service external ID",
                external,
                "runner-installation:",
            ),
            problems,
        );
    }
    if let Some(fingerprint) = decode_fingerprint::<SystemdServiceFingerprint>(
        "systemd-service",
        identity.evidence.fingerprint.as_deref(),
        requirement,
        problems,
    ) {
        if fingerprint.schema_version != CANONICAL_RESOURCE_SCHEMA_VERSION {
            problems.push("systemd service fingerprint version is not supported".to_owned());
        }
        collect(
            canonical_absolute_path("systemd unit file", &fingerprint.unit_file).map(|_| ()),
            problems,
        );
        collect(
            validate_sha256("systemd unit content digest", &fingerprint.content_digest),
            problems,
        );
        collect(validate_linux_user(&fingerprint.service_user), problems);
        collect(
            validate_installation_id(&fingerprint.runner_installation_id),
            problems,
        );
        if let Some(external) = external {
            let expected = format!("runner-installation:{}", fingerprint.runner_installation_id);
            if external != expected {
                problems.push(
                    "systemd service external ID must match fingerprint runner installation ID"
                        .to_owned(),
                );
            }
        }
    }
}

fn validate_runner_installation_identity(
    identity: &ResourceIdentity,
    requirement: EvidenceRequirement,
    problems: &mut Vec<String>,
) {
    collect(
        canonical_absolute_path("runner installation", &identity.locator).map(|_| ()),
        problems,
    );
    if identity.evidence.external_id.is_some() {
        collect(
            validate_prefixed_u64(
                "runner installation external ID",
                identity.evidence.external_id.as_deref(),
                "github-runner:",
                true,
            ),
            problems,
        );
    }
    if let Some(fingerprint) = decode_fingerprint::<RunnerInstallationFingerprint>(
        "runner-installation",
        identity.evidence.fingerprint.as_deref(),
        requirement,
        problems,
    ) {
        if fingerprint.schema_version != CANONICAL_RESOURCE_SCHEMA_VERSION {
            problems.push("runner installation fingerprint version is not supported".to_owned());
        }
        collect(validate_server_url(&fingerprint.server_url), problems);
        collect(validate_scope(&fingerprint.scope), problems);
        collect(validate_systemd_unit(&fingerprint.service_unit), problems);
        collect(
            validate_runner_version(&fingerprint.runner_version),
            problems,
        );
    }
}

fn validate_podman_image_identity(
    identity: &ResourceIdentity,
    requirement: EvidenceRequirement,
    problems: &mut Vec<String>,
) {
    collect(validate_local_image_reference(&identity.locator), problems);
    if requirement.requires(identity.evidence.external_id.is_some()) {
        match identity.evidence.external_id.as_deref() {
            Some(external) => match external.strip_prefix("image:") {
                Some(digest) => collect(validate_sha256("Podman image digest", digest), problems),
                None => problems.push("Podman image external ID must start with image:".to_owned()),
            },
            None => problems.push("Podman image external ID is required".to_owned()),
        }
    }
    if let Some(fingerprint) = decode_fingerprint::<PodmanImageFingerprint>(
        "podman-image",
        identity.evidence.fingerprint.as_deref(),
        requirement,
        problems,
    ) {
        if fingerprint.schema_version != CANONICAL_RESOURCE_SCHEMA_VERSION {
            problems.push("Podman image fingerprint version is not supported".to_owned());
        }
        collect(
            validate_sha256("Podman build-input digest", &fingerprint.build_input_digest),
            problems,
        );
    }
}

fn validate_github_registration_identity(
    identity: &ResourceIdentity,
    requirement: EvidenceRequirement,
    problems: &mut Vec<String>,
) {
    if requirement.requires(identity.evidence.external_id.is_some()) {
        collect(
            validate_prefixed_u64(
                "GitHub registration external ID",
                identity.evidence.external_id.as_deref(),
                "github-runner:",
                true,
            ),
            problems,
        );
    }
    if let Some(fingerprint) = decode_fingerprint::<GithubRegistrationFingerprint>(
        "github-registration",
        identity.evidence.fingerprint.as_deref(),
        requirement,
        problems,
    ) {
        if fingerprint.schema_version != CANONICAL_RESOURCE_SCHEMA_VERSION {
            problems.push("GitHub registration fingerprint version is not supported".to_owned());
        }
        collect(validate_scope(&fingerprint.scope), problems);
        let prefix = format!("{}/runner:", fingerprint.scope.locator_prefix());
        match identity.locator.strip_prefix(&prefix) {
            Some(runner_name) => {
                collect(canonical_runner_name(runner_name).map(|_| ()), problems);
            }
            None => problems
                .push("GitHub registration locator does not match fingerprint scope".to_owned()),
        }
    } else if identity.locator.is_empty() || identity.locator.chars().any(char::is_control) {
        problems.push("GitHub registration locator must be non-empty and safe".to_owned());
    }
}

fn encode_fingerprint<T: Serialize>(
    kind: &str,
    value: &T,
) -> Result<String, CanonicalResourceError> {
    serde_json::to_string(value)
        .map(|json| format!("{kind}-v{CANONICAL_RESOURCE_SCHEMA_VERSION}:{json}"))
        .map_err(|error| CanonicalResourceError::single(format!("serialize fingerprint: {error}")))
}

fn decode_fingerprint<T>(
    kind: &str,
    value: Option<&str>,
    requirement: EvidenceRequirement,
    problems: &mut Vec<String>,
) -> Option<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let Some(value) = value else {
        if requirement == EvidenceRequirement::Complete {
            problems.push(format!("{kind} fingerprint is required"));
        }
        return None;
    };
    let prefix = format!("{kind}-v{CANONICAL_RESOURCE_SCHEMA_VERSION}:");
    let Some(json) = value.strip_prefix(&prefix) else {
        problems.push(format!("{kind} fingerprint must start with {prefix}"));
        return None;
    };
    match serde_json::from_str::<T>(json) {
        Ok(decoded) => {
            match encode_fingerprint(kind, &decoded) {
                Ok(canonical) if canonical == value => {}
                Ok(_) => problems.push(format!("{kind} fingerprint is not canonical JSON")),
                Err(error) => problems.extend(error.problems),
            }
            Some(decoded)
        }
        Err(error) => {
            problems.push(format!("invalid {kind} fingerprint: {error}"));
            None
        }
    }
}

fn validate_scope(scope: &GithubScope) -> Result<(), CanonicalResourceError> {
    match scope {
        GithubScope::Repository { id, slug } => {
            if *id == 0 {
                return Err(CanonicalResourceError::single(
                    "repository scope ID must be greater than zero",
                ));
            }
            if canonical_repository_slug(slug)? != *slug {
                return Err(CanonicalResourceError::single(
                    "repository scope slug must use canonical lowercase OWNER/REPOSITORY",
                ));
            }
        }
        GithubScope::Organization { id, login } => {
            if *id == 0 {
                return Err(CanonicalResourceError::single(
                    "organization scope ID must be greater than zero",
                ));
            }
            if canonical_github_name("organization login", login)? != *login {
                return Err(CanonicalResourceError::single(
                    "organization login must use canonical lowercase form",
                ));
            }
        }
    }
    Ok(())
}

fn canonical_repository_slug(value: &str) -> Result<String, CanonicalResourceError> {
    let parts = value.split('/').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(CanonicalResourceError::single(
            "repository scope must be exact OWNER/REPOSITORY",
        ));
    }
    let owner = canonical_github_name("repository owner", parts[0])?;
    let repository = canonical_github_name("repository name", parts[1])?;
    Ok(format!("{owner}/{repository}"))
}

fn canonical_github_name(field: &str, value: &str) -> Result<String, CanonicalResourceError> {
    if value.is_empty()
        || value.len() > MAX_NAME_LEN
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(CanonicalResourceError::single(format!(
            "{field} must contain only ASCII letters, digits, '.', '_', or '-'",
        )));
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_runner_name(value: &str) -> Result<String, CanonicalResourceError> {
    if value.is_empty()
        || value.len() > MAX_NAME_LEN
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(CanonicalResourceError::single(
            "runner name must contain only ASCII letters, digits, '.', '_', or '-'",
        ));
    }
    Ok(value.to_owned())
}

fn canonical_absolute_path(field: &str, value: &str) -> Result<String, CanonicalResourceError> {
    if value.len() > MAX_PATH_LEN
        || value.is_empty()
        || !value.starts_with('/')
        || value.contains("//")
        || (value.len() > 1 && value.ends_with('/'))
        || value.chars().any(char::is_control)
    {
        return Err(CanonicalResourceError::single(format!(
            "{field} must be a canonical absolute path",
        )));
    }
    let path = Path::new(value);
    if path
        .components()
        .any(|component| !matches!(component, Component::RootDir | Component::Normal(_)))
    {
        return Err(CanonicalResourceError::single(format!(
            "{field} must not contain path aliases",
        )));
    }
    Ok(value.to_owned())
}

fn validate_linux_user(value: &str) -> Result<(), CanonicalResourceError> {
    let mut bytes = value.bytes();
    let first_valid = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte == b'_');
    if !first_valid
        || value.len() > 32
        || !bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
    {
        return Err(CanonicalResourceError::single(
            "Linux user must be a safe lowercase username",
        ));
    }
    Ok(())
}

fn validate_installation_id(value: &str) -> Result<(), CanonicalResourceError> {
    if !(16..=80).contains(&value.len())
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(CanonicalResourceError::single(
            "installation ID must be 16 to 80 lowercase ASCII letters, digits, or '-'",
        ));
    }
    Ok(())
}

fn validate_mode(mode: u32) -> Result<(), CanonicalResourceError> {
    if mode > 0o7777 || mode & 0o700 == 0 || mode & 0o002 != 0 {
        return Err(CanonicalResourceError::single(
            "directory mode must be an octal permission value without world-write access",
        ));
    }
    Ok(())
}

fn validate_systemd_unit(value: &str) -> Result<(), CanonicalResourceError> {
    if !value.ends_with(".service") || value.len() > 255 || value.contains('/') {
        return Err(CanonicalResourceError::single(
            "systemd unit must be an exact escaped .service name",
        ));
    }
    let stem = &value[..value.len() - ".service".len()];
    if stem.is_empty() {
        return Err(CanonicalResourceError::single(
            "systemd unit stem must not be empty",
        ));
    }
    let bytes = stem.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'.' | b'@' | b'-') {
            index += 1;
        } else if byte == b'\\'
            && index + 3 < bytes.len()
            && bytes[index + 1] == b'x'
            && bytes[index + 2].is_ascii_hexdigit()
            && bytes[index + 3].is_ascii_hexdigit()
        {
            index += 4;
        } else {
            return Err(CanonicalResourceError::single(
                "systemd unit contains an unescaped unsafe character",
            ));
        }
    }
    Ok(())
}

fn validate_sha256(field: &str, value: &str) -> Result<(), CanonicalResourceError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(CanonicalResourceError::single(format!(
            "{field} must use sha256:<64 lowercase hex>",
        )));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CanonicalResourceError::single(format!(
            "{field} must use sha256:<64 lowercase hex>",
        )));
    }
    Ok(())
}

fn validate_server_url(value: &str) -> Result<(), CanonicalResourceError> {
    if !value.starts_with("https://")
        || value.len() > 512
        || value.ends_with('/')
        || value.contains('?')
        || value.contains('#')
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(CanonicalResourceError::single(
            "runner server URL must be canonical HTTPS without query, fragment, or trailing slash",
        ));
    }
    let host_and_path = &value["https://".len()..];
    let host = host_and_path.split('/').next().unwrap_or_default();
    if host.is_empty() || host != host.to_ascii_lowercase() {
        return Err(CanonicalResourceError::single(
            "runner server URL host must be non-empty lowercase ASCII",
        ));
    }
    Ok(())
}

fn validate_runner_version(value: &str) -> Result<(), CanonicalResourceError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(CanonicalResourceError::single(
            "runner version must contain three numeric components",
        ));
    }
    Ok(())
}

fn validate_local_image_reference(value: &str) -> Result<(), CanonicalResourceError> {
    if !value.starts_with("localhost/")
        || !value.contains(':')
        || value.contains('@')
        || value != value.to_ascii_lowercase()
        || value.len() > 255
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':')
        })
    {
        return Err(CanonicalResourceError::single(
            "Podman image locator must be an exact lowercase localhost/name:tag reference",
        ));
    }
    Ok(())
}

fn validate_prefixed_u64(
    field: &str,
    value: Option<&str>,
    prefix: &str,
    nonzero: bool,
) -> Result<(), CanonicalResourceError> {
    let Some(value) = value else {
        return Err(CanonicalResourceError::single(format!(
            "{field} is required",
        )));
    };
    let Some(number) = value.strip_prefix(prefix) else {
        return Err(CanonicalResourceError::single(format!(
            "{field} must start with {prefix}",
        )));
    };
    let parsed = number.parse::<u64>().map_err(|_| {
        CanonicalResourceError::single(format!("{field} must contain a decimal integer"))
    })?;
    if (nonzero && parsed == 0) || parsed.to_string() != number {
        return Err(CanonicalResourceError::single(format!(
            "{field} must contain a canonical decimal integer",
        )));
    }
    Ok(())
}

fn validate_prefixed_token(
    field: &str,
    value: Option<&str>,
    prefix: &str,
) -> Result<(), CanonicalResourceError> {
    let Some(value) = value else {
        return Err(CanonicalResourceError::single(format!(
            "{field} is required",
        )));
    };
    let Some(token) = value.strip_prefix(prefix) else {
        return Err(CanonicalResourceError::single(format!(
            "{field} must start with {prefix}",
        )));
    };
    validate_installation_id(token)
}

fn collect(result: Result<(), CanonicalResourceError>, problems: &mut Vec<String>) {
    if let Err(error) = result {
        problems.extend(error.problems);
    }
}

#[cfg(test)]
mod tests {
    use crate::ownership::{ResourceEvidence, ResourceIdentity, ResourceKind};

    use super::{
        AccountPolicy, CANONICAL_RESOURCE_SCHEMA_VERSION, GithubScope, policy, validate_identity,
    };

    const DIGEST_A: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn path_aliases_are_rejected() {
        let error = ResourceIdentity::directory(
            "/var/lib/smolrunner/../foreign",
            "0123456789abcdef",
            1000,
            1000,
            0o750,
        )
        .expect_err("parent traversal must fail");
        assert!(
            error
                .problems
                .iter()
                .any(|problem| problem.contains("alias"))
        );
    }

    #[test]
    fn repository_scope_is_case_canonical() {
        let upper = GithubScope::repository(42, "Example/Project").expect("valid scope");
        let lower = GithubScope::repository(42, "example/project").expect("valid scope");
        assert_eq!(upper, lower);
    }

    #[test]
    fn escaped_systemd_names_are_accepted_and_raw_spaces_are_rejected() {
        ResourceIdentity::systemd_service(
            "actions.runner.example-project.host\\x2done.service",
            "/etc/systemd/system/actions.runner.example-project.host\\x2done.service",
            DIGEST_A,
            "project-runner",
            "0123456789abcdef",
        )
        .expect("escaped unit is canonical");

        ResourceIdentity::systemd_service(
            "actions runner.service",
            "/etc/systemd/system/actions runner.service",
            DIGEST_A,
            "project-runner",
            "0123456789abcdef",
        )
        .expect_err("raw space must fail");
    }

    #[test]
    fn mutable_image_tag_without_digest_is_invalid() {
        let identity = ResourceIdentity::new(
            ResourceKind::PodmanImage,
            "localhost/example-ci:latest",
            ResourceEvidence::none(),
        );
        let problems = validate_identity(&identity);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("external ID"))
        );
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("fingerprint"))
        );
    }

    #[test]
    fn duplicate_runner_names_are_disambiguated_by_scope_and_runner_id() {
        let first = ResourceIdentity::github_runner_registration(
            GithubScope::repository(42, "example/project").expect("scope"),
            "project-vps",
            100,
        )
        .expect("first registration");
        let second = ResourceIdentity::github_runner_registration(
            GithubScope::repository(43, "example/other").expect("scope"),
            "project-vps",
            101,
        )
        .expect("second registration");
        assert_ne!(first.locator, second.locator);
        assert_ne!(first.evidence.external_id, second.evidence.external_id);
    }

    #[test]
    fn repository_transfer_changes_fingerprint_but_not_numeric_scope_locator() {
        let before = ResourceIdentity::github_runner_registration(
            GithubScope::repository(42, "old-owner/project").expect("scope"),
            "project-vps",
            100,
        )
        .expect("before transfer");
        let after = ResourceIdentity::github_runner_registration(
            GithubScope::repository(42, "new-owner/project").expect("scope"),
            "project-vps",
            100,
        )
        .expect("after transfer");
        assert_eq!(before.locator, after.locator);
        assert_ne!(before.evidence.fingerprint, after.evidence.fingerprint);
    }

    #[test]
    fn runner_reregistration_changes_external_id() {
        let scope = GithubScope::repository(42, "example/project").expect("scope");
        let old = ResourceIdentity::github_runner_registration(scope.clone(), "project-vps", 100)
            .expect("old registration");
        let new = ResourceIdentity::github_runner_registration(scope, "project-vps", 101)
            .expect("new registration");
        assert_eq!(old.locator, new.locator);
        assert_ne!(old.evidence.external_id, new.evidence.external_id);
    }

    #[test]
    fn constructors_emit_valid_kind_specific_identity() {
        let identities = [
            ResourceIdentity::linux_user(
                "project-runner",
                1001,
                1001,
                "/var/lib/project-runner",
                "/usr/sbin/nologin",
                AccountPolicy::Service,
            )
            .expect("Linux user"),
            ResourceIdentity::directory(
                "/var/lib/smolrunner/installations/0123456789abcdef",
                "0123456789abcdef",
                0,
                0,
                0o750,
            )
            .expect("directory"),
            ResourceIdentity::systemd_service(
                "actions.runner.example-project.host.service",
                "/etc/systemd/system/actions.runner.example-project.host.service",
                DIGEST_A,
                "project-runner",
                "0123456789abcdef",
            )
            .expect("service"),
            ResourceIdentity::runner_installation(
                "/opt/smolrunner/runners/project",
                "https://github.com",
                GithubScope::repository(42, "example/project").expect("scope"),
                Some(100),
                "actions.runner.example-project.host.service",
                "2.336.0",
            )
            .expect("runner installation"),
            ResourceIdentity::podman_image("localhost/project-ci:1", DIGEST_A, DIGEST_B)
                .expect("image"),
            ResourceIdentity::github_runner_registration(
                GithubScope::repository(42, "example/project").expect("scope"),
                "project-vps",
                100,
            )
            .expect("registration"),
        ];

        for identity in identities {
            assert!(
                validate_identity(&identity).is_empty(),
                "invalid identity: {identity:?}"
            );
        }
    }

    #[test]
    fn policies_define_required_observation_lanes_and_survival() {
        let registration = policy(ResourceKind::GithubRunnerRegistration);
        assert_eq!(
            registration.observation_lanes,
            [crate::journal::ExecutionLane::Github]
        );
        assert!(registration.survival.host_restore);
        assert!(!registration.survival.repository_transfer);
        assert!(!registration.survival.runner_reregistration);

        let image = policy(ResourceKind::PodmanImage);
        assert_eq!(
            image.observation_lanes,
            [crate::journal::ExecutionLane::RunnerUser]
        );
        assert!(!image.survival.host_restore);
    }

    #[test]
    fn fingerprint_schema_is_versioned() {
        let identity = ResourceIdentity::linux_user(
            "project-runner",
            1001,
            1001,
            "/var/lib/project-runner",
            "/usr/sbin/nologin",
            AccountPolicy::Service,
        )
        .expect("Linux user");
        assert!(
            identity.evidence.fingerprint.as_deref().is_some_and(
                |value| value.contains(&format!("-v{CANONICAL_RESOURCE_SCHEMA_VERSION}:"))
            )
        );
    }
}
