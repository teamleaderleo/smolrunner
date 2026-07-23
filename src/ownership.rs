use std::fmt;

use serde::Serialize;

use crate::manifest::{Manifest, RunnerScope};

pub const OWNERSHIP_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectIdentity {
    pub repository: String,
    pub runner_scope: RunnerScope,
    pub runner_user: String,
}

impl From<&Manifest> for ProjectIdentity {
    fn from(manifest: &Manifest) -> Self {
        Self {
            repository: manifest.repository.clone(),
            runner_scope: manifest.runner.scope,
            runner_user: manifest.runner.user.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    LinuxUser,
    Directory,
    SystemdService,
    RunnerInstallation,
    PodmanImage,
    GithubRunnerRegistration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceEvidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

impl ResourceEvidence {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            external_id: None,
            fingerprint: None,
        }
    }

    #[must_use]
    pub fn external_id(value: impl Into<String>) -> Self {
        Self {
            external_id: Some(value.into()),
            fingerprint: None,
        }
    }

    #[must_use]
    pub fn fingerprint(value: impl Into<String>) -> Self {
        Self {
            external_id: None,
            fingerprint: Some(value.into()),
        }
    }

    #[must_use]
    pub fn both(external_id: impl Into<String>, fingerprint: impl Into<String>) -> Self {
        Self {
            external_id: Some(external_id.into()),
            fingerprint: Some(fingerprint.into()),
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.external_id.is_none() && self.fingerprint.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceIdentity {
    pub kind: ResourceKind,
    pub locator: String,
    pub evidence: ResourceEvidence,
}

impl ResourceIdentity {
    #[must_use]
    pub fn new(
        kind: ResourceKind,
        locator: impl Into<String>,
        evidence: ResourceEvidence,
    ) -> Self {
        Self {
            kind,
            locator: locator.into(),
            evidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnershipMarker {
    pub schema_version: u8,
    pub installation_id: String,
    pub project: ProjectIdentity,
    pub resource: ResourceIdentity,
}

impl OwnershipMarker {
    #[must_use]
    pub fn new(
        installation_id: impl Into<String>,
        project: ProjectIdentity,
        resource: ResourceIdentity,
    ) -> Self {
        Self {
            schema_version: OWNERSHIP_SCHEMA_VERSION,
            installation_id: installation_id.into(),
            project,
            resource,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnershipContext {
    pub installation_id: String,
    pub project: ProjectIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObservedResource {
    pub identity: ResourceIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<OwnershipMarker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipClass {
    Managed,
    Adoptable,
    Foreign,
    Conflicting,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnershipAssessment {
    pub class: OwnershipClass,
    pub reasons: Vec<String>,
}

impl OwnershipAssessment {
    fn new(class: OwnershipClass, reason: impl Into<String>) -> Self {
        Self {
            class,
            reasons: vec![reason.into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnershipValidationError {
    pub problems: Vec<String>,
}

impl fmt::Display for OwnershipValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "ownership identity validation failed")?;
        for problem in &self.problems {
            writeln!(formatter, "- {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for OwnershipValidationError {}

/// Classify one existing resource without mutating or adopting it.
///
/// # Errors
///
/// Returns a validation error when project, installation, locator, marker version, or evidence
/// values are structurally unsafe or incomplete.
pub fn classify(
    context: &OwnershipContext,
    desired: &ResourceIdentity,
    observed: &ObservedResource,
) -> Result<OwnershipAssessment, OwnershipValidationError> {
    validate(context, desired, observed)?;

    if desired.kind != observed.identity.kind || desired.locator != observed.identity.locator {
        return Ok(OwnershipAssessment::new(
            OwnershipClass::Conflicting,
            "the desired locator is occupied by a different resource identity",
        ));
    }

    if let Some(marker) = &observed.marker {
        return classify_marked(context, desired, observed, marker);
    }

    classify_unmarked(desired, observed)
}

fn classify_marked(
    context: &OwnershipContext,
    desired: &ResourceIdentity,
    observed: &ObservedResource,
    marker: &OwnershipMarker,
) -> Result<OwnershipAssessment, OwnershipValidationError> {
    if marker.schema_version != OWNERSHIP_SCHEMA_VERSION {
        return Ok(OwnershipAssessment::new(
            OwnershipClass::Unknown,
            format!(
                "ownership marker version {} is not understood",
                marker.schema_version
            ),
        ));
    }

    if marker.project != context.project || marker.installation_id != context.installation_id {
        return Ok(OwnershipAssessment::new(
            OwnershipClass::Foreign,
            "the resource is marked for another project or SmolRunner installation",
        ));
    }

    if marker.resource.kind != observed.identity.kind
        || marker.resource.locator != observed.identity.locator
        || marker.resource.kind != desired.kind
        || marker.resource.locator != desired.locator
    {
        return Ok(OwnershipAssessment::new(
            OwnershipClass::Conflicting,
            "the ownership marker does not describe the observed and desired resource locator",
        ));
    }

    match compare_required_evidence(&marker.resource.evidence, &observed.identity.evidence) {
        EvidenceMatch::Mismatch => {
            return Ok(OwnershipAssessment::new(
                OwnershipClass::Conflicting,
                "observed evidence conflicts with the ownership marker",
            ));
        }
        EvidenceMatch::Missing => {
            return Ok(OwnershipAssessment::new(
                OwnershipClass::Unknown,
                "the ownership marker matches, but required observed evidence is unavailable",
            ));
        }
        EvidenceMatch::Exact => {}
    }

    match compare_required_evidence(&desired.evidence, &observed.identity.evidence) {
        EvidenceMatch::Mismatch => Ok(OwnershipAssessment::new(
            OwnershipClass::Conflicting,
            "the managed resource no longer matches desired immutable evidence",
        )),
        EvidenceMatch::Missing => Ok(OwnershipAssessment::new(
            OwnershipClass::Unknown,
            "desired immutable evidence cannot be verified from the current observation",
        )),
        EvidenceMatch::Exact => Ok(OwnershipAssessment::new(
            OwnershipClass::Managed,
            "the marker, project, installation, locator, and required evidence match exactly",
        )),
    }
}

fn classify_unmarked(
    desired: &ResourceIdentity,
    observed: &ObservedResource,
) -> Result<OwnershipAssessment, OwnershipValidationError> {
    if desired.evidence.is_empty() {
        return Ok(OwnershipAssessment::new(
            OwnershipClass::Unknown,
            "matching names or locators without immutable evidence do not establish ownership",
        ));
    }

    match compare_required_evidence(&desired.evidence, &observed.identity.evidence) {
        EvidenceMatch::Exact => Ok(OwnershipAssessment::new(
            OwnershipClass::Adoptable,
            "the unmarked resource matches exact desired evidence and requires explicit adoption",
        )),
        EvidenceMatch::Missing => Ok(OwnershipAssessment::new(
            OwnershipClass::Unknown,
            "the unmarked resource lacks enough evidence for safe adoption",
        )),
        EvidenceMatch::Mismatch => Ok(OwnershipAssessment::new(
            OwnershipClass::Conflicting,
            "the unmarked resource conflicts with desired immutable evidence",
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceMatch {
    Exact,
    Missing,
    Mismatch,
}

fn compare_required_evidence(required: &ResourceEvidence, observed: &ResourceEvidence) -> EvidenceMatch {
    let external = compare_optional_required(&required.external_id, &observed.external_id);
    let fingerprint = compare_optional_required(&required.fingerprint, &observed.fingerprint);

    if external == EvidenceMatch::Mismatch || fingerprint == EvidenceMatch::Mismatch {
        EvidenceMatch::Mismatch
    } else if external == EvidenceMatch::Missing || fingerprint == EvidenceMatch::Missing {
        EvidenceMatch::Missing
    } else {
        EvidenceMatch::Exact
    }
}

fn compare_optional_required(required: &Option<String>, observed: &Option<String>) -> EvidenceMatch {
    match (required, observed) {
        (None, _) => EvidenceMatch::Exact,
        (Some(_), None) => EvidenceMatch::Missing,
        (Some(required), Some(observed)) if required == observed => EvidenceMatch::Exact,
        (Some(_), Some(_)) => EvidenceMatch::Mismatch,
    }
}

fn validate(
    context: &OwnershipContext,
    desired: &ResourceIdentity,
    observed: &ObservedResource,
) -> Result<(), OwnershipValidationError> {
    let mut problems = Vec::new();
    validate_token("installation_id", &context.installation_id, &mut problems);
    validate_project("project", &context.project, &mut problems);
    validate_resource("desired", desired, &mut problems);
    validate_resource("observed", &observed.identity, &mut problems);

    if let Some(marker) = &observed.marker {
        validate_token("marker.installation_id", &marker.installation_id, &mut problems);
        validate_project("marker.project", &marker.project, &mut problems);
        validate_resource("marker.resource", &marker.resource, &mut problems);
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(OwnershipValidationError { problems })
    }
}

fn validate_project(field: &str, project: &ProjectIdentity, problems: &mut Vec<String>) {
    if project.repository.is_empty() || !project.repository.contains('/') {
        problems.push(format!("{field}.repository must be OWNER/REPOSITORY"));
    }
    if project.runner_user.is_empty() {
        problems.push(format!("{field}.runner_user must not be empty"));
    }
}

fn validate_resource(field: &str, resource: &ResourceIdentity, problems: &mut Vec<String>) {
    if resource.locator.is_empty() || resource.locator.chars().any(char::is_control) {
        problems.push(format!("{field}.locator must be non-empty and contain no control characters"));
    }
    for (name, value) in [
        ("external_id", &resource.evidence.external_id),
        ("fingerprint", &resource.evidence.fingerprint),
    ] {
        if value
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.chars().any(char::is_control))
        {
            problems.push(format!(
                "{field}.evidence.{name} must be non-empty and contain no control characters"
            ));
        }
    }
}

fn validate_token(field: &str, value: &str, problems: &mut Vec<String>) {
    let valid = (16..=80).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !valid {
        problems.push(format!(
            "{field} must be 16 to 80 lowercase ASCII letters, digits, or '-'"
        ));
    }
}

#[cfg(test)]
mod tests {
    use crate::manifest::RunnerScope;

    use super::{
        ObservedResource, OwnershipClass, OwnershipContext, OwnershipMarker, ProjectIdentity,
        ResourceEvidence, ResourceIdentity, ResourceKind, classify,
    };

    fn project(repository: &str) -> ProjectIdentity {
        ProjectIdentity {
            repository: repository.to_owned(),
            runner_scope: RunnerScope::Repository,
            runner_user: "project-runner".to_owned(),
        }
    }

    fn context() -> OwnershipContext {
        OwnershipContext {
            installation_id: "0123456789abcdef".to_owned(),
            project: project("example/project"),
        }
    }

    fn runner(external_id: Option<&str>) -> ResourceIdentity {
        ResourceIdentity::new(
            ResourceKind::GithubRunnerRegistration,
            "repository:example/project/runner:project-vps",
            external_id.map_or_else(ResourceEvidence::none, ResourceEvidence::external_id),
        )
    }

    #[test]
    fn exact_marker_and_evidence_are_managed() {
        let context = context();
        let desired = runner(Some("runner-id-42"));
        let marker = OwnershipMarker::new(
            context.installation_id.clone(),
            context.project.clone(),
            desired.clone(),
        );
        let observed = ObservedResource {
            identity: desired.clone(),
            marker: Some(marker),
        };

        assert_eq!(
            classify(&context, &desired, &observed)
                .expect("valid ownership")
                .class,
            OwnershipClass::Managed
        );
    }

    #[test]
    fn matching_name_with_foreign_marker_is_protected() {
        let context = context();
        let desired = runner(Some("runner-id-42"));
        let marker = OwnershipMarker::new(
            "fedcba9876543210",
            project("other/project"),
            desired.clone(),
        );
        let observed = ObservedResource {
            identity: desired.clone(),
            marker: Some(marker),
        };

        assert_eq!(
            classify(&context, &desired, &observed)
                .expect("valid ownership")
                .class,
            OwnershipClass::Foreign
        );
    }

    #[test]
    fn unmarked_exact_evidence_is_only_adoptable() {
        let context = context();
        let desired = runner(Some("runner-id-42"));
        let observed = ObservedResource {
            identity: desired.clone(),
            marker: None,
        };

        assert_eq!(
            classify(&context, &desired, &observed)
                .expect("valid ownership")
                .class,
            OwnershipClass::Adoptable
        );
    }

    #[test]
    fn names_without_evidence_remain_unknown() {
        let context = context();
        let desired = runner(None);
        let observed = ObservedResource {
            identity: desired.clone(),
            marker: None,
        };

        assert_eq!(
            classify(&context, &desired, &observed)
                .expect("valid ownership")
                .class,
            OwnershipClass::Unknown
        );
    }

    #[test]
    fn missing_required_observation_remains_unknown() {
        let context = context();
        let desired = runner(Some("runner-id-42"));
        let observed = ObservedResource {
            identity: runner(None),
            marker: None,
        };

        assert_eq!(
            classify(&context, &desired, &observed)
                .expect("valid ownership")
                .class,
            OwnershipClass::Unknown
        );
    }

    #[test]
    fn mismatched_immutable_evidence_is_conflicting() {
        let context = context();
        let desired = runner(Some("runner-id-42"));
        let observed = ObservedResource {
            identity: runner(Some("runner-id-99")),
            marker: None,
        };

        assert_eq!(
            classify(&context, &desired, &observed)
                .expect("valid ownership")
                .class,
            OwnershipClass::Conflicting
        );
    }

    #[test]
    fn matching_marker_with_missing_observed_evidence_is_unknown() {
        let context = context();
        let desired = runner(Some("runner-id-42"));
        let marker = OwnershipMarker::new(
            context.installation_id.clone(),
            context.project.clone(),
            desired.clone(),
        );
        let observed = ObservedResource {
            identity: runner(None),
            marker: Some(marker),
        };

        assert_eq!(
            classify(&context, &desired, &observed)
                .expect("valid ownership")
                .class,
            OwnershipClass::Unknown
        );
    }

    #[test]
    fn invalid_installation_identity_fails_before_classification() {
        let mut context = context();
        context.installation_id = "TOO SHORT".to_owned();
        let desired = runner(Some("runner-id-42"));
        let observed = ObservedResource {
            identity: desired.clone(),
            marker: None,
        };

        let error = classify(&context, &desired, &observed).expect_err("invalid context");
        assert!(
            error
                .problems
                .iter()
                .any(|problem| problem.contains("installation_id"))
        );
    }
}
