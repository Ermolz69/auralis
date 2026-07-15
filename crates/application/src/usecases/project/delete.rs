use std::sync::Arc;

use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::job_runtime_control::JobRuntimeControlPort;
use ports::transaction::{CommitProjectDelete, StorageUnitOfWork};

pub struct DeleteProjectRequest {
    pub project_id: ProjectId,
}

pub struct DeleteProjectUseCase {
    storage_uow: Arc<dyn StorageUnitOfWork>,
    job_runtime: Arc<dyn JobRuntimeControlPort>,
}

impl DeleteProjectUseCase {
    pub fn new(
        storage_uow: Arc<dyn StorageUnitOfWork>,
        job_runtime: Arc<dyn JobRuntimeControlPort>,
    ) -> Self {
        Self {
            storage_uow,
            job_runtime,
        }
    }

    pub async fn execute(&self, request: DeleteProjectRequest) -> Result<(), ApplicationError> {
        let project_id = &request.project_id;

        // 1. Execute transaction
        let commit = CommitProjectDelete {
            project_id: project_id.clone(),
        };

        let result = self
            .storage_uow
            .commit_project_delete(commit)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to delete project: {}", e),
            })?;

        // 2. Perform runtime cleanup
        if let Err(e) = self
            .job_runtime
            .cancel_and_evict_jobs(&result.deleted_job_ids)
            .await
        {
            tracing::error!(
                project_id = %project_id,
                "CRITICAL: Failed to cleanup runtime jobs for project: {}",
                e
            );
            // We do not return an error to the UI because the project is already persistently deleted.
        }

        Ok(())
    }
}
