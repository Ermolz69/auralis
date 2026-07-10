use std::sync::Arc;

use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::transaction::{CommitProjectDelete, StorageUnitOfWork};

pub struct DeleteProjectRequest {
    pub project_id: ProjectId,
}

pub struct DeleteProjectUseCase {
    artifact_index: Arc<dyn ArtifactIndex>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
}

impl DeleteProjectUseCase {
    pub fn new(
        artifact_index: Arc<dyn ArtifactIndex>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
    ) -> Self {
        Self {
            artifact_index,
            storage_uow,
        }
    }

    pub async fn execute(&self, request: DeleteProjectRequest) -> Result<(), ApplicationError> {
        let project_id = &request.project_id;

        // 1. List all artifacts for the project
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
