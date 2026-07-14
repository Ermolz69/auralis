use async_trait::async_trait;
use std::sync::Arc;

use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitPipelineStart, CommitPipelineStartFailure, CommitProjectDelete,
    CommitStagedArtifactWrite, CommitTranscriptImport, StorageUnitOfWork,
};

use super::job_repository::InMemoryJobRepository;
use super::project_repository::InMemoryProjectRepository;
use ports::repository::{JobRepository, ProjectRepository};

#[derive(Clone)]
pub struct InMemoryStorageUnitOfWork {
    project_repo: Arc<InMemoryProjectRepository>,
    job_repo: Arc<InMemoryJobRepository>,
    artifact_index: Arc<dyn ports::artifact_index::ArtifactIndex>,
    artifact_store: Arc<dyn ports::storage::ArtifactStore>,
}

impl InMemoryStorageUnitOfWork {
    pub fn new(
        project_repo: Arc<InMemoryProjectRepository>,
        job_repo: Arc<InMemoryJobRepository>,
        artifact_index: Arc<dyn ports::artifact_index::ArtifactIndex>,
        artifact_store: Arc<dyn ports::storage::ArtifactStore>,
    ) -> Self {
        Self {
            project_repo,
            job_repo,
            artifact_index,
            artifact_store,
        }
    }
}

#[async_trait]
impl StorageUnitOfWork for InMemoryStorageUnitOfWork {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        self.project_repo.save(&command.project).await?;
        Ok(())
    }

    async fn commit_staged_artifact_write(
        &self,
        command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        // Synchronously finalize artifact for dev mode
        self.artifact_store
            .finalize_staged_artifact(&command.staging_key, &command.final_key)
            .await?;

        let mut artifact = command.artifact;
        artifact.state = domain::media::ArtifactState::Ready;

        self.artifact_index
            .add(&command.project_id, &artifact)
            .await?;

        Ok(())
    }

    async fn commit_managed_source_import(
        &self,
        command: ports::transaction::CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        // Synchronously finalize artifact for dev mode
        self.artifact_store
            .finalize_staged_artifact(&command.staging_key, &command.final_key)
            .await?;

        let mut artifact = command.artifact;
        artifact.state = domain::media::ArtifactState::Ready;

        self.project_repo.save(&command.project).await?;
        self.artifact_index
            .add(command.project.id(), &artifact)
            .await?;

        Ok(())
    }

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        self.project_repo.delete(&command.project_id).await?;
        Ok(())
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        self.job_repo.save(&command.job).await?;
        Ok(())
    }

    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError> {
        command.validate()?;

        // Best effort atomic write for dev adapter. Real atomicity relies on SQLite implementation.
        self.project_repo.save(&command.project).await?;
        self.job_repo.save(&command.job).await?;
        Ok(())
    }

    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        command.validate()?;

        // Best effort atomic write for dev adapter. Real atomicity relies on SQLite implementation.
        self.project_repo.save(&command.project).await?;
        self.job_repo.save(&command.job).await?;
        Ok(())
    }

    async fn commit_terminal_job_update(
        &self,
        command: ports::transaction::CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        self.job_repo.save(&command.job).await?;
        // InMemoryAdapter doesn't have an outbox right now, so we just return
        Ok(())
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ports::transaction::ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        let mut project = self
            .project_repo
            .get(&command.project_id)
            .await?
            .ok_or_else(|| PortError::Unexpected {
                message: "Project not found".to_string(),
            })?;

        let res = project
            .apply_terminal_transition(&command.job_id, command.outcome)
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;

        if res == domain::project::status::TerminalTransitionResult::Applied {
            self.project_repo.save(&project).await?;
        }

        Ok(res)
    }
}
