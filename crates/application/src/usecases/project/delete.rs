use std::sync::Arc;

use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::repository::JobRepository;
use ports::transaction::{CommitProjectDelete, StorageUnitOfWork};

pub struct DeleteProjectRequest {
    pub project_id: ProjectId,
}

pub struct DeleteProjectUseCase {
    artifact_index: Arc<dyn ArtifactIndex>,
    job_repository: Arc<dyn JobRepository>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
}

impl DeleteProjectUseCase {
    pub fn new(
        artifact_index: Arc<dyn ArtifactIndex>,
        job_repository: Arc<dyn JobRepository>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
    ) -> Self {
        Self {
            artifact_index,
            job_repository,
            storage_uow,
        }
    }

    pub async fn execute(&self, request: DeleteProjectRequest) -> Result<(), ApplicationError> {
        let project_id = &request.project_id;

        // 1. Check for active jobs
        let jobs = self
            .job_repository
            .list_by_project(project_id)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to list jobs: {}", e),
            })?;

        let has_active_jobs = jobs.into_iter().any(|j| {
            matches!(
                j.status(),
                domain::job::JobStatus::Pending | domain::job::JobStatus::Running
            )
        });

        if has_active_jobs {
            return Err(ApplicationError::InvalidOperation {
                message: "Cannot delete project with active jobs".to_string(),
            });
        }

        // 2. List all artifacts for the project
        let artifacts = self.artifact_index.list_by_project(project_id).await?;

        // 2. Execute transaction
        let commit = CommitProjectDelete {
            project_id: project_id.clone(),
            artifacts,
        };

        self.storage_uow
            .commit_project_delete(commit)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to delete project: {}", e),
            })?;

        Ok(())
    }
}
