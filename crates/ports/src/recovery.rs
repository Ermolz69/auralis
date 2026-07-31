use async_trait::async_trait;
use domain::job::JobId;
use domain::job::{Job, JobStatus};
use domain::project::{Project, ProjectStatus};

use crate::error::PortError;

pub struct RecoverySnapshot {
    pub processing_projects: Vec<Project>,
    pub linked_jobs: Vec<Job>,
    pub active_jobs: Vec<Job>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecoveryApplyResult {
    Applied,
    AlreadyApplied,
}

pub struct FailInterruptedPairCommand {
    pub project: Project,
    pub job: Job,
    pub expected_project_status: ProjectStatus,
    pub expected_active_job_id: JobId,
    pub expected_job_status: JobStatus, // e.g. Pending or Running
    pub expected_last_terminal_job_id: Option<JobId>,
}

pub struct ReconcileTerminalPairCommand {
    pub project: Project,
    pub job: Job,
    pub expected_project_status: ProjectStatus, // Processing
    pub expected_active_job_id: JobId,
    pub expected_job_status: JobStatus, // e.g. Completed, Failed, Cancelled
    pub expected_last_terminal_job_id: Option<JobId>,
}

pub struct FailLegacyPairFallbackCommand {
    pub project: Project,
    pub job: Job,
    pub expected_project_status: ProjectStatus, // Processing
    // active_job_id must be NULL
    pub expected_job_status: JobStatus, // Pending or Running
    pub expected_last_terminal_job_id: Option<JobId>,
}

pub struct FailProjectWithMissingLinkedJobCommand {
    pub project: Project,
    pub expected_project_status: ProjectStatus, // Processing
    pub expected_active_job_id: JobId,
    pub expected_last_terminal_job_id: Option<JobId>,
}

pub struct FailLegacyProjectWithoutJobCommand {
    pub project: Project,
    pub expected_project_status: ProjectStatus, // Processing
    // active_job_id must be NULL
    pub expected_last_terminal_job_id: Option<JobId>,
}

pub struct FailOrphanJobCommand {
    pub job: Job,
    pub expected_job_status: JobStatus, // Pending or Running
                                        // job must not be referenced by any current Processing project's active_job_id
}

#[async_trait]
pub trait RecoveryStorage: Send + Sync {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError>;

    async fn commit_failed_interrupted_pair(
        &self,
        cmd: FailInterruptedPairCommand,
    ) -> Result<RecoveryApplyResult, PortError>;

    async fn commit_reconciled_terminal_pair(
        &self,
        cmd: ReconcileTerminalPairCommand,
    ) -> Result<RecoveryApplyResult, PortError>;

    async fn commit_legacy_pair_fallback(
        &self,
        cmd: FailLegacyPairFallbackCommand,
    ) -> Result<RecoveryApplyResult, PortError>;

    async fn commit_failed_project_with_missing_linked_job(
        &self,
        cmd: FailProjectWithMissingLinkedJobCommand,
    ) -> Result<RecoveryApplyResult, PortError>;

    async fn commit_failed_legacy_project_without_job(
        &self,
        cmd: FailLegacyProjectWithoutJobCommand,
    ) -> Result<RecoveryApplyResult, PortError>;

    async fn commit_failed_orphan_job(
        &self,
        cmd: FailOrphanJobCommand,
    ) -> Result<RecoveryApplyResult, PortError>;
}
