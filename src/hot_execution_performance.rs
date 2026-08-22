use std::fmt;

use serde::Serialize;

pub const HOT_EXECUTION_PERFORMANCE_SCHEMA_VERSION: u8 = 1;
pub const MAX_HOT_EXECUTION_DURATION_MILLIS: u64 = 7 * 24 * 60 * 60 * 1_000;
pub const MAX_HOT_EXECUTION_CPU_TIME_MILLIS: u64 = 365 * 24 * 60 * 60 * 1_000;
pub const MAX_HOT_EXECUTION_OBSERVED_BYTES: u64 = 1 << 50;

const MAX_TOKEN_BYTES: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum HotExecutionPerformanceDocumentType {
    HotExecutionPerformanceReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotExecutionPerformanceAuthority {
    ObservationOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotExecutionMode {
    ColdDisposable,
    PreparedDisposable,
    WarmPoolDisposable,
    ResidentAfterIdle,
    ResidentImmediate,
    ResidentTaskLoop,
}

impl HotExecutionMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ColdDisposable => "cold_disposable",
            Self::PreparedDisposable => "prepared_disposable",
            Self::WarmPoolDisposable => "warm_pool_disposable",
            Self::ResidentAfterIdle => "resident_after_idle",
            Self::ResidentImmediate => "resident_immediate",
            Self::ResidentTaskLoop => "resident_task_loop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotSandboxState {
    Cold,
    Prepared,
    ResidentHit,
    Resumed,
    Reset,
}

impl HotSandboxState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Prepared => "prepared",
            Self::ResidentHit => "resident_hit",
            Self::Resumed => "resumed",
            Self::Reset => "reset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotRepositoryState {
    Cold,
    ObjectHit,
    CheckoutHit,
    ResidentHit,
    TaskFork,
    Reset,
}

impl HotRepositoryState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::ObjectHit => "object_hit",
            Self::CheckoutHit => "checkout_hit",
            Self::ResidentHit => "resident_hit",
            Self::TaskFork => "task_fork",
            Self::Reset => "reset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotDependencyState {
    Cold,
    StoreHit,
    EnvironmentHit,
    ResidentHit,
    Reset,
}

impl HotDependencyState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::StoreHit => "store_hit",
            Self::EnvironmentHit => "environment_hit",
            Self::ResidentHit => "resident_hit",
            Self::Reset => "reset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotBuildState {
    Cold,
    CompilerCacheHit,
    IncrementalHit,
    ResidentHit,
    Reset,
}

impl HotBuildState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::CompilerCacheHit => "compiler_cache_hit",
            Self::IncrementalHit => "incremental_hit",
            Self::ResidentHit => "resident_hit",
            Self::Reset => "reset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotIndexServiceState {
    Cold,
    ResidentHit,
    Reset,
    Unavailable,
}

impl HotIndexServiceState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::ResidentHit => "resident_hit",
            Self::Reset => "reset",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HotExecutionResultClass {
    Succeeded,
    Failed,
    Canceled,
    ResetRequired,
    Unknown,
}

impl HotExecutionResultClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
            Self::ResetRequired => "reset_required",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotExecutionHeat {
    sandbox: HotSandboxState,
    repository: HotRepositoryState,
    dependency: HotDependencyState,
    build: HotBuildState,
    index_service: HotIndexServiceState,
}

impl HotExecutionHeat {
    #[must_use]
    pub const fn new(
        sandbox: HotSandboxState,
        repository: HotRepositoryState,
        dependency: HotDependencyState,
        build: HotBuildState,
        index_service: HotIndexServiceState,
    ) -> Self {
        Self {
            sandbox,
            repository,
            dependency,
            build,
            index_service,
        }
    }

    #[must_use]
    pub const fn sandbox(&self) -> HotSandboxState {
        self.sandbox
    }

    #[must_use]
    pub const fn repository(&self) -> HotRepositoryState {
        self.repository
    }

    #[must_use]
    pub const fn dependency(&self) -> HotDependencyState {
        self.dependency
    }

    #[must_use]
    pub const fn build(&self) -> HotBuildState {
        self.build
    }

    #[must_use]
    pub const fn index_service(&self) -> HotIndexServiceState {
        self.index_service
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotExecutionMilestones {
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox_ready_millis: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_ready_millis: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependency_ready_millis: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_useful_command_millis: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_relevant_result_millis: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    final_relevant_result_millis: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    residency_transition_millis: Option<u64>,
}

impl HotExecutionMilestones {
    /// Construct run-relative milestone observations measured from one monotonic start point.
    ///
    /// Present milestones must be monotonic in the declared sequence. Missing milestones remain
    /// explicit gaps and carry no inferred timing evidence.
    ///
    /// # Errors
    ///
    /// Returns a bounded error when one observation exceeds the supported experiment window or a
    /// later declared milestone precedes an earlier observed milestone.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sandbox_ready_millis: Option<u64>,
        repository_ready_millis: Option<u64>,
        dependency_ready_millis: Option<u64>,
        first_useful_command_millis: Option<u64>,
        first_relevant_result_millis: Option<u64>,
        final_relevant_result_millis: Option<u64>,
        residency_transition_millis: Option<u64>,
    ) -> Result<Self, HotExecutionPerformanceError> {
        let values = [
            sandbox_ready_millis,
            repository_ready_millis,
            dependency_ready_millis,
            first_useful_command_millis,
            first_relevant_result_millis,
            final_relevant_result_millis,
            residency_transition_millis,
        ];
        let mut previous = None;
        for value in values.into_iter().flatten() {
            if value > MAX_HOT_EXECUTION_DURATION_MILLIS {
                return Err(duration_out_of_range());
            }
            if previous.is_some_and(|previous| value < previous) {
                return Err(invalid_milestone_order());
            }
            previous = Some(value);
        }
        Ok(Self {
            sandbox_ready_millis,
            repository_ready_millis,
            dependency_ready_millis,
            first_useful_command_millis,
            first_relevant_result_millis,
            final_relevant_result_millis,
            residency_transition_millis,
        })
    }

    fn validate_total(&self, total_elapsed_millis: u64) -> Result<(), HotExecutionPerformanceError> {
        for value in [
            self.sandbox_ready_millis,
            self.repository_ready_millis,
            self.dependency_ready_millis,
            self.first_useful_command_millis,
            self.first_relevant_result_millis,
            self.final_relevant_result_millis,
            self.residency_transition_millis,
        ]
        .into_iter()
        .flatten()
        {
            if value > total_elapsed_millis {
                return Err(milestone_after_total());
            }
        }
        Ok(())
    }

    #[must_use]
    pub const fn first_useful_command_millis(&self) -> Option<u64> {
        self.first_useful_command_millis
    }

    #[must_use]
    pub const fn first_relevant_result_millis(&self) -> Option<u64> {
        self.first_relevant_result_millis
    }

    #[must_use]
    pub const fn final_relevant_result_millis(&self) -> Option<u64> {
        self.final_relevant_result_millis
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotExecutionStorageObservation {
    #[serde(skip_serializing_if = "Option::is_none")]
    guest_logical_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guest_allocated_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_backing_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_materialization_bytes: Option<u64>,
}

impl HotExecutionStorageObservation {
    /// Construct distinct logical, guest-allocation, host-backing, and task-materialization facts.
    ///
    /// # Errors
    ///
    /// Returns a bounded error when one observed byte count exceeds the supported receipt range.
    pub fn new(
        guest_logical_bytes: Option<u64>,
        guest_allocated_bytes: Option<u64>,
        host_backing_bytes: Option<u64>,
        task_materialization_bytes: Option<u64>,
    ) -> Result<Self, HotExecutionPerformanceError> {
        for value in [
            guest_logical_bytes,
            guest_allocated_bytes,
            host_backing_bytes,
            task_materialization_bytes,
        ]
        .into_iter()
        .flatten()
        {
            if value > MAX_HOT_EXECUTION_OBSERVED_BYTES {
                return Err(observation_out_of_range());
            }
        }
        Ok(Self {
            guest_logical_bytes,
            guest_allocated_bytes,
            host_backing_bytes,
            task_materialization_bytes,
        })
    }

    #[must_use]
    pub const fn guest_logical_bytes(&self) -> Option<u64> {
        self.guest_logical_bytes
    }

    #[must_use]
    pub const fn guest_allocated_bytes(&self) -> Option<u64> {
        self.guest_allocated_bytes
    }

    #[must_use]
    pub const fn host_backing_bytes(&self) -> Option<u64> {
        self.host_backing_bytes
    }

    #[must_use]
    pub const fn task_materialization_bytes(&self) -> Option<u64> {
        self.task_materialization_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotExecutionResourceObservation {
    #[serde(skip_serializing_if = "Option::is_none")]
    peak_guest_memory_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_memory_delta_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_time_millis: Option<u64>,
}

impl HotExecutionResourceObservation {
    /// Construct bounded memory and CPU observations for one execution sample.
    ///
    /// # Errors
    ///
    /// Returns a bounded error for observations outside the supported receipt range.
    pub fn new(
        peak_guest_memory_bytes: Option<u64>,
        host_memory_delta_bytes: Option<i64>,
        cpu_time_millis: Option<u64>,
    ) -> Result<Self, HotExecutionPerformanceError> {
        if peak_guest_memory_bytes.is_some_and(|value| value > MAX_HOT_EXECUTION_OBSERVED_BYTES) {
            return Err(observation_out_of_range());
        }
        if host_memory_delta_bytes.is_some_and(|value| {
            value < -(MAX_HOT_EXECUTION_OBSERVED_BYTES as i64)
                || value > MAX_HOT_EXECUTION_OBSERVED_BYTES as i64
        }) {
            return Err(observation_out_of_range());
        }
        if cpu_time_millis.is_some_and(|value| value > MAX_HOT_EXECUTION_CPU_TIME_MILLIS) {
            return Err(cpu_time_out_of_range());
        }
        Ok(Self {
            peak_guest_memory_bytes,
            host_memory_delta_bytes,
            cpu_time_millis,
        })
    }

    #[must_use]
    pub const fn peak_guest_memory_bytes(&self) -> Option<u64> {
        self.peak_guest_memory_bytes
    }

    #[must_use]
    pub const fn host_memory_delta_bytes(&self) -> Option<i64> {
        self.host_memory_delta_bytes
    }

    #[must_use]
    pub const fn cpu_time_millis(&self) -> Option<u64> {
        self.cpu_time_millis
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotExecutionPerformanceIdentity {
    workload_id: HotExecutionPerformanceToken,
    project_id: HotExecutionPerformanceToken,
    candidate_id: HotExecutionPerformanceToken,
    backend_id: HotExecutionPerformanceToken,
    host_class: HotExecutionPerformanceToken,
    resource_profile: HotExecutionPerformanceToken,
}

impl HotExecutionPerformanceIdentity {
    /// Construct content-minimised comparison identity for one hot-execution sample.
    ///
    /// The candidate identifies the optimization under test independently from the execution
    /// backend, so a filesystem, worktree, cache, or backend experiment can change one dimension at
    /// a time.
    ///
    /// # Errors
    ///
    /// Returns a bounded error unless every identity is a compact lowercase ASCII token.
    pub fn new(
        workload_id: &str,
        project_id: &str,
        candidate_id: &str,
        backend_id: &str,
        host_class: &str,
        resource_profile: &str,
    ) -> Result<Self, HotExecutionPerformanceError> {
        Ok(Self {
            workload_id: HotExecutionPerformanceToken::parse(workload_id)?,
            project_id: HotExecutionPerformanceToken::parse(project_id)?,
            candidate_id: HotExecutionPerformanceToken::parse(candidate_id)?,
            backend_id: HotExecutionPerformanceToken::parse(backend_id)?,
            host_class: HotExecutionPerformanceToken::parse(host_class)?,
            resource_profile: HotExecutionPerformanceToken::parse(resource_profile)?,
        })
    }

    #[must_use]
    pub fn workload_id(&self) -> &str {
        self.workload_id.as_str()
    }

    #[must_use]
    pub fn project_id(&self) -> &str {
        self.project_id.as_str()
    }

    #[must_use]
    pub fn candidate_id(&self) -> &str {
        self.candidate_id.as_str()
    }

    #[must_use]
    pub fn backend_id(&self) -> &str {
        self.backend_id.as_str()
    }

    #[must_use]
    pub fn host_class(&self) -> &str {
        self.host_class.as_str()
    }

    #[must_use]
    pub fn resource_profile(&self) -> &str {
        self.resource_profile.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotExecutionPerformanceReceipt {
    document_type: HotExecutionPerformanceDocumentType,
    schema_version: u8,
    authority: HotExecutionPerformanceAuthority,
    identity: HotExecutionPerformanceIdentity,
    execution_mode: HotExecutionMode,
    total_elapsed_millis: u64,
    milestones: HotExecutionMilestones,
    heat: HotExecutionHeat,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<HotExecutionStorageObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<HotExecutionResourceObservation>,
    result: HotExecutionResultClass,
}

impl HotExecutionPerformanceReceipt {
    /// Build one observation-only performance receipt from already-owned benchmark boundaries.
    ///
    /// This document carries comparison evidence only. Lifecycle, verification, cache-publication,
    /// cleanup, and mutation authority remain in their existing typed boundaries.
    ///
    /// # Errors
    ///
    /// Returns a bounded error when the total duration exceeds the supported window or a milestone
    /// occurs after the declared terminal duration.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: HotExecutionPerformanceIdentity,
        execution_mode: HotExecutionMode,
        total_elapsed_millis: u64,
        milestones: HotExecutionMilestones,
        heat: HotExecutionHeat,
        storage: Option<HotExecutionStorageObservation>,
        resources: Option<HotExecutionResourceObservation>,
        result: HotExecutionResultClass,
    ) -> Result<Self, HotExecutionPerformanceError> {
        if total_elapsed_millis > MAX_HOT_EXECUTION_DURATION_MILLIS {
            return Err(duration_out_of_range());
        }
        milestones.validate_total(total_elapsed_millis)?;
        Ok(Self {
            document_type: HotExecutionPerformanceDocumentType::HotExecutionPerformanceReceipt,
            schema_version: HOT_EXECUTION_PERFORMANCE_SCHEMA_VERSION,
            authority: HotExecutionPerformanceAuthority::ObservationOnly,
            identity,
            execution_mode,
            total_elapsed_millis,
            milestones,
            heat,
            storage,
            resources,
            result,
        })
    }

    #[must_use]
    pub const fn schema_version(&self) -> u8 {
        self.schema_version
    }

    #[must_use]
    pub const fn authority(&self) -> HotExecutionPerformanceAuthority {
        self.authority
    }

    #[must_use]
    pub const fn identity(&self) -> &HotExecutionPerformanceIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn execution_mode(&self) -> HotExecutionMode {
        self.execution_mode
    }

    #[must_use]
    pub const fn total_elapsed_millis(&self) -> u64 {
        self.total_elapsed_millis
    }

    #[must_use]
    pub const fn milestones(&self) -> &HotExecutionMilestones {
        &self.milestones
    }

    #[must_use]
    pub const fn heat(&self) -> &HotExecutionHeat {
        &self.heat
    }

    #[must_use]
    pub const fn storage(&self) -> Option<&HotExecutionStorageObservation> {
        self.storage.as_ref()
    }

    #[must_use]
    pub const fn resources(&self) -> Option<&HotExecutionResourceObservation> {
        self.resources.as_ref()
    }

    #[must_use]
    pub const fn result(&self) -> HotExecutionResultClass {
        self.result
    }

    /// Render one stable human summary from the same typed receipt used for JSON output.
    #[must_use]
    pub fn render_human(&self) -> String {
        let mut output = format!(
            "hot execution performance\nworkload: {}\nproject: {}\ncandidate: {}\nmode: {}\nbackend: {}\nhost: {}\nresource profile: {}\nresult: {}\ntotal: {} ms\nfirst useful command: {}\nfirst relevant result: {}\nfinal relevant result: {}\n",
            self.identity.workload_id(),
            self.identity.project_id(),
            self.identity.candidate_id(),
            self.execution_mode.as_str(),
            self.identity.backend_id(),
            self.identity.host_class(),
            self.identity.resource_profile(),
            self.result.as_str(),
            self.total_elapsed_millis,
            render_optional_millis(self.milestones.first_useful_command_millis()),
            render_optional_millis(self.milestones.first_relevant_result_millis()),
            render_optional_millis(self.milestones.final_relevant_result_millis()),
        );
        output.push_str(&format!(
            "heat: sandbox={} repository={} dependency={} build={} index_service={}\n",
            self.heat.sandbox.as_str(),
            self.heat.repository.as_str(),
            self.heat.dependency.as_str(),
            self.heat.build.as_str(),
            self.heat.index_service.as_str(),
        ));
        output
    }

    /// Render deterministic pretty JSON from this bounded receipt.
    ///
    /// # Errors
    ///
    /// Returns only if serialization of the fixed receipt model fails.
    pub fn render_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
struct HotExecutionPerformanceToken(String);

impl HotExecutionPerformanceToken {
    fn parse(value: &str) -> Result<Self, HotExecutionPerformanceError> {
        let Some(first) = value.bytes().next() else {
            return Err(invalid_token());
        };
        if value.len() > MAX_TOKEN_BYTES
            || !(first.is_ascii_lowercase() || first.is_ascii_digit())
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-' | b':')
            })
        {
            return Err(invalid_token());
        }
        Ok(Self(value.to_owned()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotExecutionPerformanceError {
    code: &'static str,
    message: &'static str,
}

impl HotExecutionPerformanceError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for HotExecutionPerformanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for HotExecutionPerformanceError {}

fn render_optional_millis(value: Option<u64>) -> String {
    value.map_or_else(|| "unknown".to_owned(), |value| format!("{value} ms"))
}

const fn error(code: &'static str, message: &'static str) -> HotExecutionPerformanceError {
    HotExecutionPerformanceError { code, message }
}

const fn invalid_token() -> HotExecutionPerformanceError {
    error(
        "invalid_performance_token",
        "performance identity must be a bounded lowercase ASCII token",
    )
}

const fn duration_out_of_range() -> HotExecutionPerformanceError {
    error(
        "performance_duration_out_of_range",
        "performance duration exceeds the supported observation window",
    )
}

const fn cpu_time_out_of_range() -> HotExecutionPerformanceError {
    error(
        "performance_cpu_time_out_of_range",
        "performance CPU time exceeds the supported observation range",
    )
}

const fn invalid_milestone_order() -> HotExecutionPerformanceError {
    error(
        "performance_milestone_order_invalid",
        "performance milestones must be monotonic",
    )
}

const fn milestone_after_total() -> HotExecutionPerformanceError {
    error(
        "performance_milestone_after_total",
        "performance milestone exceeds the total elapsed duration",
    )
}

const fn observation_out_of_range() -> HotExecutionPerformanceError {
    error(
        "performance_observation_out_of_range",
        "performance resource observation exceeds the supported range",
    )
}

#[cfg(test)]
mod tests {
    use super::{
        HotBuildState, HotDependencyState, HotExecutionHeat, HotExecutionMilestones,
        HotExecutionMode, HotExecutionPerformanceAuthority, HotExecutionPerformanceIdentity,
        HotExecutionPerformanceReceipt, HotExecutionResourceObservation, HotExecutionResultClass,
        HotExecutionStorageObservation, HotIndexServiceState, HotRepositoryState, HotSandboxState,
        MAX_HOT_EXECUTION_DURATION_MILLIS, MAX_HOT_EXECUTION_OBSERVED_BYTES,
    };

    fn representative_receipt() -> HotExecutionPerformanceReceipt {
        HotExecutionPerformanceReceipt::new(
            HotExecutionPerformanceIdentity::new(
                "quarry-edit-test",
                "quarry",
                "project-disk-xfs-reflink",
                "lima-vz",
                "apple-silicon-24g",
                "medium-4c-8g",
            )
            .expect("identity is valid"),
            HotExecutionMode::ResidentTaskLoop,
            2_800,
            HotExecutionMilestones::new(
                Some(0),
                Some(5),
                Some(8),
                Some(12),
                Some(2_420),
                Some(2_760),
                Some(2_800),
            )
            .expect("milestones are monotonic"),
            HotExecutionHeat::new(
                HotSandboxState::ResidentHit,
                HotRepositoryState::TaskFork,
                HotDependencyState::EnvironmentHit,
                HotBuildState::IncrementalHit,
                HotIndexServiceState::ResidentHit,
            ),
            Some(
                HotExecutionStorageObservation::new(
                    Some(9_000_000_000),
                    Some(2_000_000_000),
                    Some(1_200_000_000),
                    Some(14_000_000),
                )
                .expect("storage observation is valid"),
            ),
            Some(
                HotExecutionResourceObservation::new(
                    Some(3_000_000_000),
                    Some(400_000_000),
                    Some(6_900),
                )
                .expect("resource observation is valid"),
            ),
            HotExecutionResultClass::Succeeded,
        )
        .expect("receipt is valid")
    }

    #[test]
    fn representative_receipt_renders_stable_human_and_json() {
        let receipt = representative_receipt();
        assert_eq!(receipt.schema_version(), 1);
        assert_eq!(
            receipt.authority(),
            HotExecutionPerformanceAuthority::ObservationOnly
        );
        assert_eq!(receipt.execution_mode(), HotExecutionMode::ResidentTaskLoop);
        assert_eq!(receipt.total_elapsed_millis(), 2_800);
        assert_eq!(
            receipt.render_human(),
            "hot execution performance\nworkload: quarry-edit-test\nproject: quarry\ncandidate: project-disk-xfs-reflink\nmode: resident_task_loop\nbackend: lima-vz\nhost: apple-silicon-24g\nresource profile: medium-4c-8g\nresult: succeeded\ntotal: 2800 ms\nfirst useful command: 12 ms\nfirst relevant result: 2420 ms\nfinal relevant result: 2760 ms\nheat: sandbox=resident_hit repository=task_fork dependency=environment_hit build=incremental_hit index_service=resident_hit\n"
        );

        let first = receipt.render_json().expect("receipt serializes");
        let second = receipt.render_json().expect("receipt serializes again");
        assert_eq!(first, second);
        let parsed: serde_json::Value = serde_json::from_str(&first).expect("JSON parses");
        assert_eq!(parsed["authority"], "observation_only");
        assert_eq!(
            parsed["identity"]["candidate_id"],
            "project-disk-xfs-reflink"
        );
        assert_eq!(parsed["identity"]["backend_id"], "lima-vz");
        assert_eq!(parsed["milestones"]["first_useful_command_millis"], 12);
        assert_eq!(parsed["storage"]["guest_logical_bytes"], 9_000_000_000_u64);
        assert_eq!(parsed["storage"]["guest_allocated_bytes"], 2_000_000_000_u64);
        assert_eq!(parsed["storage"]["host_backing_bytes"], 1_200_000_000_u64);
        assert_eq!(parsed["storage"]["task_materialization_bytes"], 14_000_000_u64);
    }

    #[test]
    fn identity_rejects_free_form_or_path_like_values_without_echoing_them() {
        let secret_like = "/private/tmp/project secret";
        let error = HotExecutionPerformanceIdentity::new(
            secret_like,
            "quarry",
            "baseline",
            "lima-vz",
            "apple-silicon",
            "medium",
        )
        .expect_err("free-form identity is refused");
        assert_eq!(error.code(), "invalid_performance_token");
        assert!(!error.to_string().contains(secret_like));
        assert!(!format!("{error:?}").contains(secret_like));
    }

    #[test]
    fn milestone_order_and_terminal_bound_are_checked() {
        let order_error = HotExecutionMilestones::new(
            Some(10),
            Some(9),
            None,
            None,
            None,
            None,
            None,
        )
        .expect_err("decreasing milestone is refused");
        assert_eq!(
            order_error.code(),
            "performance_milestone_order_invalid"
        );

        let milestones = HotExecutionMilestones::new(
            Some(1),
            None,
            None,
            Some(20),
            None,
            None,
            None,
        )
        .expect("milestones are internally monotonic");
        let error = HotExecutionPerformanceReceipt::new(
            HotExecutionPerformanceIdentity::new(
                "smolrunner-edit-test",
                "smolrunner",
                "baseline",
                "lima-vz",
                "apple-silicon",
                "medium",
            )
            .expect("identity is valid"),
            HotExecutionMode::ResidentImmediate,
            10,
            milestones,
            HotExecutionHeat::new(
                HotSandboxState::ResidentHit,
                HotRepositoryState::ResidentHit,
                HotDependencyState::ResidentHit,
                HotBuildState::ResidentHit,
                HotIndexServiceState::ResidentHit,
            ),
            None,
            None,
            HotExecutionResultClass::Succeeded,
        )
        .expect_err("milestone after terminal duration is refused");
        assert_eq!(error.code(), "performance_milestone_after_total");
    }

    #[test]
    fn duration_and_storage_observations_are_bounded() {
        let milestone_error = HotExecutionMilestones::new(
            Some(MAX_HOT_EXECUTION_DURATION_MILLIS + 1),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect_err("oversized milestone is refused");
        assert_eq!(
            milestone_error.code(),
            "performance_duration_out_of_range"
        );

        let storage_error = HotExecutionStorageObservation::new(
            Some(MAX_HOT_EXECUTION_OBSERVED_BYTES + 1),
            None,
            None,
            None,
        )
        .expect_err("oversized storage observation is refused");
        assert_eq!(
            storage_error.code(),
            "performance_observation_out_of_range"
        );
    }

    #[test]
    fn missing_optional_evidence_stays_absent_and_human_output_says_unknown() {
        let receipt = HotExecutionPerformanceReceipt::new(
            HotExecutionPerformanceIdentity::new(
                "cold-baseline",
                "smolrunner",
                "baseline",
                "lima-vz",
                "apple-silicon",
                "small",
            )
            .expect("identity is valid"),
            HotExecutionMode::ColdDisposable,
            200,
            HotExecutionMilestones::new(None, None, None, None, None, None, None)
                .expect("empty milestone evidence is allowed"),
            HotExecutionHeat::new(
                HotSandboxState::Cold,
                HotRepositoryState::Cold,
                HotDependencyState::Cold,
                HotBuildState::Cold,
                HotIndexServiceState::Unavailable,
            ),
            None,
            None,
            HotExecutionResultClass::Unknown,
        )
        .expect("receipt is valid");

        assert!(receipt.storage().is_none());
        assert!(receipt.resources().is_none());
        assert!(receipt.render_human().contains("first useful command: unknown"));
        let json = receipt.render_json().expect("receipt serializes");
        assert!(!json.contains("\"storage\""));
        assert!(!json.contains("\"resources\""));
    }
}
