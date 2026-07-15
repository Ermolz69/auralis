pub mod artifact;
pub mod pipeline_start;
pub mod project_delete;
pub mod terminal_lifecycle;

pub use artifact::*;
pub use pipeline_start::*;
pub use project_delete::*;
pub use terminal_lifecycle::*;

use crate::error::PortError;
use async_trait::async_trait;

#[async_trait]
pub trait StorageUnitOfWork: Send + Sync {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError>;
    async fn commit_staged_artifact_write(
        &self,
        command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError>;
    async fn commit_managed_source_import(
        &self,
        command: CommitManagedSourceImport,
    ) -> Result<(), PortError>;
    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError>;
    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError>;
    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError>;
    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError>;
    async fn commit_terminal_job_update(
        &self,
        command: CommitTerminalJobUpdate,
    ) -> Result<(), PortError>;
    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError>;
}

#[async_trait]
impl<T: ?Sized + StorageUnitOfWork> StorageUnitOfWork for std::sync::Arc<T> {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        (**self).commit_transcript_import(command).await
    }
    async fn commit_staged_artifact_write(
        &self,
        command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        (**self).commit_staged_artifact_write(command).await
    }
    async fn commit_managed_source_import(
        &self,
        command: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        (**self).commit_managed_source_import(command).await
    }
    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        (**self).commit_project_delete(command).await
    }
    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        (**self).commit_job_update(command).await
    }
    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError> {
        (**self).commit_pipeline_start(command).await
    }
    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        (**self).commit_pipeline_start_failure(command).await
    }
    async fn commit_terminal_job_update(
        &self,
        command: CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        (**self).commit_terminal_job_update(command).await
    }
    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        (**self)
            .apply_terminal_lifecycle_conditionally(command)
            .await
    }
}
