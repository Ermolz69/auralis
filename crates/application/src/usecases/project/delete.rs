use std::sync::Arc;

use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::job_runtime_control::JobRuntimeControlPort;
use ports::transaction::{CommitProjectDelete, StorageUnitOfWork};

use crate::usecases::project::lifecycle::ProjectLifecycleLocks;

pub struct DeleteProjectRequest {
    pub project_id: ProjectId,
}

pub struct DeleteProjectUseCase {
    storage_uow: Arc<dyn StorageUnitOfWork>,
    job_runtime: Arc<dyn JobRuntimeControlPort>,
    locks: Arc<ProjectLifecycleLocks>,
}

impl DeleteProjectUseCase {
    pub fn new(
        storage_uow: Arc<dyn StorageUnitOfWork>,
        job_runtime: Arc<dyn JobRuntimeControlPort>,
        locks: Arc<ProjectLifecycleLocks>,
    ) -> Self {
        Self {
            storage_uow,
            job_runtime,
            locks,
        }
    }

    pub async fn execute(&self, request: DeleteProjectRequest) -> Result<(), ApplicationError> {
        let project_id = &request.project_id;

        let lock_arc = self.locks.get_lock(project_id);
        let _lock = lock_arc.lock().await;

        // 1. Execute transaction
        let commit = CommitProjectDelete {
            project_id: project_id.clone(),
        };

        let result = self.storage_uow.commit_project_delete(commit).await?;

        // 2. Perform runtime cleanup
        match self
            .job_runtime
            .cancel_and_evict_jobs(&result.deleted_job_ids)
            .await
        {
            Ok(report) => {
                let unconfirmed: Vec<_> = report
                    .jobs
                    .iter()
                    .filter(|(_, status)| {
                        matches!(
                            status,
                            ports::job_runtime_control::JobCleanupStatus::AbortUnconfirmed
                        )
                    })
                    .map(|(id, _)| id.to_string())
                    .collect();

                if !unconfirmed.is_empty() {
                    tracing::error!(
                        project_id = %project_id,
                        unconfirmed_jobs = ?unconfirmed,
                        "CRITICAL: Failed to confirm termination of the following jobs after abort"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    project_id = %project_id,
                    "CRITICAL: Failed to cleanup runtime jobs for project: {}",
                    e
                );
                // We do not return an error to the UI because the project is already persistently deleted.
            }
        }

        Ok(())
    }
}
