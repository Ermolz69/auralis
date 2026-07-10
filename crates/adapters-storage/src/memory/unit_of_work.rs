use async_trait::async_trait;
use std::sync::Arc;

use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitProjectDelete, CommitStagedArtifactWrite, CommitTranscriptImport,
    StorageUnitOfWork,
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

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        self.project_repo.delete(&command.project_id).await?;
        Ok(())
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        self.job_repo.save(&command.job).await?;
        Ok(())
    }
}
