use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::manifest::RunnerScope;
use crate::ownership::{
    OWNERSHIP_SCHEMA_VERSION, OwnershipMarker, ProjectIdentity, ResourceEvidence, ResourceIdentity,
    ResourceKind,
};
use crate::state::InstallationId;

pub const STATE_DOCUMENT_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "document_type", content = "document", rename_all = "snake_case")]
pub enum StateDocument {
    Project(ProjectStateDocument),
    Resource(ResourceStateDocument),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectStateDocument {
    schema_version: u8,
    installation_id: InstallationId,
    project: ProjectIdentity,
}

impl ProjectStateDocument {
    #[must_use]
    pub fn schema_version(&self) -> u8 {
        self.schema_version
    }

    #[must_use]
    pub fn installation_id(&self) -> &InstallationId {
        &self.installation_id
    }

    #[must_use]
    pub fn project(&self) -> &ProjectIdentity {
        &self.project
    }

    /// Build a validated project-state document.
    ///
    /// # Errors
    ///
    /// Returns an error when the project identity is malformed.
    pub fn new(
        installation_id: InstallationId,
        project: ProjectIdentity,
    ) -> Result<Self, StateDocumentError> {
        validate_project_identity(&project)?;
        Ok(Self {
            schema_version: STATE_DOCUMENT_SCHEMA_VERSION,
            installation_id,
            project,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceStateDocument {
    schema_version: u8,
    marker: OwnershipMarker,
}

impl ResourceStateDocument {
    #[must_use]
    pub fn schema_version(&self) -> u8 {
        self.schema_version
    }

    #[must_use]
    pub fn marker(&self) -> &OwnershipMarker {
        &self.marker
    }

    /// Build a validated resource-state document from one complete ownership marker.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown marker versions, malformed identities, or incomplete immutable
    /// evidence.
    pub fn new(marker: OwnershipMarker) -> Result<Self, StateDocumentError> {
        validate_ownership_marker(&marker)?;
        Ok(Self {
            schema_version: STATE_DOCUMENT_SCHEMA_VERSION,
            marker,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StateDocumentError {
    pub problems: Vec<String>,
}

impl StateDocumentError {
    fn single(problem: impl Into<String>) -> Self {
        Self {
            problems: vec![problem.into()],
        }
    }
}

impl fmt::Display for StateDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "state document validation failed")?;
        for problem in &self.problems {
            writeln!(formatter, "- {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for StateDocumentError {}

/// Serialize one validated state document with stable, human-readable JSON.
///
/// # Errors
///
/// Returns an error only when JSON serialization fails.
pub fn encode_state_document(document: &StateDocument) -> Result<String, StateDocumentError> {
    let mut encoded = serde_json::to_string_pretty(document).map_err(|error| {
        StateDocumentError::single(format!("state document serialization failed: {error}"))
    })?;
    encoded.push('\n');
    Ok(encoded)
}

/// Decode untrusted JSON through exact document schemas and canonical identity validation.
///
/// # Errors
///
/// Returns an error for malformed JSON, unknown fields, unsupported versions, malformed project
/// identities, or incomplete resource evidence.
pub fn decode_state_document(input: &str) -> Result<StateDocument, StateDocumentError> {
    let envelope: WireEnvelope = serde_json::from_str(input).map_err(|error| {
        StateDocumentError::single(format!("state document JSON is invalid: {error}"))
    })?;

    match envelope.document_type {
        WireDocumentType::Project => {
            let wire: WireProjectStateDocument = serde_json::from_value(envelope.document)
                .map_err(|error| {
                    StateDocumentError::single(format!("project document is invalid: {error}"))
                })?;
            wire.try_into().map(StateDocument::Project)
        }
        WireDocumentType::Resource => {
            let wire: WireResourceStateDocument = serde_json::from_value(envelope.document)
                .map_err(|error| {
                    StateDocumentError::single(format!("resource document is invalid: {error}"))
                })?;
            wire.try_into().map(StateDocument::Resource)
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireEnvelope {
    document_type: WireDocumentType,
    document: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireDocumentType {
    Project,
    Resource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireProjectStateDocument {
    schema_version: u8,
    installation_id: String,
    project: WireProjectIdentity,
}

impl TryFrom<WireProjectStateDocument> for ProjectStateDocument {
    type Error = StateDocumentError;

    fn try_from(wire: WireProjectStateDocument) -> Result<Self, Self::Error> {
        validate_state_schema(wire.schema_version)?;
        let installation_id = InstallationId::parse(&wire.installation_id)
            .map_err(|error| StateDocumentError::single(error.to_string()))?;
        Self::new(installation_id, wire.project.into())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResourceStateDocument {
    schema_version: u8,
    marker: WireOwnershipMarker,
}

impl TryFrom<WireResourceStateDocument> for ResourceStateDocument {
    type Error = StateDocumentError;

    fn try_from(wire: WireResourceStateDocument) -> Result<Self, Self::Error> {
        validate_state_schema(wire.schema_version)?;
        Self::new(wire.marker.into())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireProjectIdentity {
    repository: String,
    runner_scope: RunnerScope,
    runner_user: String,
}

impl From<WireProjectIdentity> for ProjectIdentity {
    fn from(wire: WireProjectIdentity) -> Self {
        Self {
            repository: wire.repository,
            runner_scope: wire.runner_scope,
            runner_user: wire.runner_user,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireOwnershipMarker {
    schema_version: u8,
    installation_id: String,
    project: WireProjectIdentity,
    resource: WireResourceIdentity,
}

impl From<WireOwnershipMarker> for OwnershipMarker {
    fn from(wire: WireOwnershipMarker) -> Self {
        Self {
            schema_version: wire.schema_version,
            installation_id: wire.installation_id,
            project: wire.project.into(),
            resource: wire.resource.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResourceIdentity {
    kind: WireResourceKind,
    locator: String,
    evidence: WireResourceEvidence,
}

impl From<WireResourceIdentity> for ResourceIdentity {
    fn from(wire: WireResourceIdentity) -> Self {
        ResourceIdentity::new(
            wire.kind.into(),
            wire.locator,
            ResourceEvidence {
                external_id: wire.evidence.external_id,
                fingerprint: wire.evidence.fingerprint,
            },
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireResourceKind {
    LinuxUser,
    Directory,
    SystemdService,
    RunnerInstallation,
    PodmanImage,
    GithubRunnerRegistration,
}

impl From<WireResourceKind> for ResourceKind {
    fn from(wire: WireResourceKind) -> Self {
        match wire {
            WireResourceKind::LinuxUser => Self::LinuxUser,
            WireResourceKind::Directory => Self::Directory,
            WireResourceKind::SystemdService => Self::SystemdService,
            WireResourceKind::RunnerInstallation => Self::RunnerInstallation,
            WireResourceKind::PodmanImage => Self::PodmanImage,
            WireResourceKind::GithubRunnerRegistration => Self::GithubRunnerRegistration,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResourceEvidence {
    #[serde(default)]
    external_id: Option<String>,
    #[serde(default)]
    fingerprint: Option<String>,
}

fn validate_state_schema(schema_version: u8) -> Result<(), StateDocumentError> {
    if schema_version == STATE_DOCUMENT_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(StateDocumentError::single(format!(
            "state document schema version {schema_version} is not supported"
        )))
    }
}

fn validate_project_identity(project: &ProjectIdentity) -> Result<(), StateDocumentError> {
    let mut problems = Vec::new();
    let repository_parts = project.repository.split('/').collect::<Vec<_>>();
    if repository_parts.len() != 2
        || repository_parts
            .iter()
            .any(|part| part.is_empty() || !is_github_name(part))
    {
        problems.push("project.repository must be exact OWNER/REPOSITORY".to_owned());
    }
    if !is_linux_user(&project.runner_user) {
        problems.push("project.runner_user must be a safe lowercase Linux username".to_owned());
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(StateDocumentError { problems })
    }
}

fn validate_ownership_marker(marker: &OwnershipMarker) -> Result<(), StateDocumentError> {
    let mut problems = Vec::new();
    if marker.schema_version != OWNERSHIP_SCHEMA_VERSION {
        problems.push(format!(
            "ownership marker schema version {} is not supported",
            marker.schema_version
        ));
    }
    if let Err(error) = InstallationId::parse(&marker.installation_id) {
        problems.push(error.to_string());
    }
    if let Err(error) = validate_project_identity(&marker.project) {
        problems.extend(error.problems);
    }
    problems.extend(
        crate::resource::validate_identity(&marker.resource)
            .into_iter()
            .map(|problem| format!("marker.resource: {problem}")),
    );

    if problems.is_empty() {
        Ok(())
    } else {
        Err(StateDocumentError { problems })
    }
}

fn is_github_name(value: &str) -> bool {
    value.len() <= 100
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn is_linux_user(value: &str) -> bool {
    let mut bytes = value.bytes();
    let first_valid = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte == b'_');
    first_valid
        && value.len() <= 32
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::manifest::RunnerScope;
    use crate::ownership::{OwnershipMarker, ProjectIdentity, ResourceIdentity};
    use crate::resource::AccountPolicy;
    use crate::state::InstallationId;

    use super::{
        ProjectStateDocument, ResourceStateDocument, StateDocument, decode_state_document,
        encode_state_document,
    };

    fn project() -> ProjectIdentity {
        ProjectIdentity {
            repository: "example/project".to_owned(),
            runner_scope: RunnerScope::Repository,
            runner_user: "project-runner".to_owned(),
        }
    }

    fn marker() -> OwnershipMarker {
        OwnershipMarker::new(
            "0123456789abcdef",
            project(),
            ResourceIdentity::linux_user(
                "project-runner",
                1001,
                1001,
                "/var/lib/project-runner",
                "/usr/sbin/nologin",
                AccountPolicy::Service,
            )
            .expect("canonical Linux user"),
        )
    }

    #[test]
    fn project_document_round_trips_through_strict_json() {
        let document = StateDocument::Project(
            ProjectStateDocument::new(
                InstallationId::parse("0123456789abcdef").expect("installation ID"),
                project(),
            )
            .expect("valid project document"),
        );
        let encoded = encode_state_document(&document).expect("encode document");
        assert_eq!(
            decode_state_document(&encoded).expect("decode document"),
            document
        );
    }

    #[test]
    fn resource_document_round_trips_complete_marker_evidence() {
        let document = StateDocument::Resource(
            ResourceStateDocument::new(marker()).expect("valid resource document"),
        );
        let encoded = encode_state_document(&document).expect("encode document");
        assert_eq!(
            decode_state_document(&encoded).expect("decode document"),
            document
        );
    }

    #[test]
    fn unknown_top_level_and_secret_fields_are_rejected() {
        let value = json!({
            "document_type": "project",
            "document": {
                "schema_version": 1,
                "installation_id": "0123456789abcdef",
                "project": {
                    "repository": "example/project",
                    "runner_scope": "repository",
                    "runner_user": "project-runner"
                }
            },
            "token": "must-never-persist"
        });
        decode_state_document(&value.to_string()).expect_err("unknown field must fail");
    }

    #[test]
    fn forward_state_schema_fails_closed() {
        let value = json!({
            "document_type": "project",
            "document": {
                "schema_version": 2,
                "installation_id": "0123456789abcdef",
                "project": {
                    "repository": "example/project",
                    "runner_scope": "repository",
                    "runner_user": "project-runner"
                }
            }
        });
        let error =
            decode_state_document(&value.to_string()).expect_err("forward version must fail");
        assert!(
            error
                .problems
                .iter()
                .any(|problem| problem.contains("version 2"))
        );
    }

    #[test]
    fn malformed_project_identity_fails_closed() {
        let value = json!({
            "document_type": "project",
            "document": {
                "schema_version": 1,
                "installation_id": "0123456789abcdef",
                "project": {
                    "repository": "missing-owner",
                    "runner_scope": "repository",
                    "runner_user": "ROOT"
                }
            }
        });
        let error = decode_state_document(&value.to_string()).expect_err("identity must fail");
        assert_eq!(error.problems.len(), 2);
    }

    #[test]
    fn resource_marker_requires_supported_version_and_complete_evidence() {
        let value = json!({
            "document_type": "resource",
            "document": {
                "schema_version": 1,
                "marker": {
                    "schema_version": 2,
                    "installation_id": "0123456789abcdef",
                    "project": {
                        "repository": "example/project",
                        "runner_scope": "repository",
                        "runner_user": "project-runner"
                    },
                    "resource": {
                        "kind": "linux_user",
                        "locator": "project-runner",
                        "evidence": {}
                    }
                }
            }
        });
        let error = decode_state_document(&value.to_string()).expect_err("marker must fail");
        assert!(
            error
                .problems
                .iter()
                .any(|problem| problem.contains("marker schema version 2"))
        );
        assert!(
            error
                .problems
                .iter()
                .any(|problem| problem.contains("external ID"))
        );
    }
}
