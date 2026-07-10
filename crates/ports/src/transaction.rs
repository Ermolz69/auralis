use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::PortError;
use domain::job::Job;
use domain::media::Artifact;
use domain::project::{Project, ProjectId};

pub struct CommitTranscriptImport {
    pub project: Project,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub temp_path_to_delete: Option<PathBuf>,
}

pub struct CommitStagedArtifactWrite {
    pub project_id: ProjectId,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub temp_path_to_delete: Option<PathBuf>,
}

pub struct CommitProjectDelete {
    pub project_id: ProjectId,
    pub artifacts: Vec<Artifact>,
}

pub struct CommitJobUpdate {
    pub job: Job,
}

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

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError>;

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError>;
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

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        (**self).commit_project_delete(command).await
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        (**self).commit_job_update(command).await
    }
}
