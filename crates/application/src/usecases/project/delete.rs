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

    pub fn execute(
        &self,
        request: DeleteProjectRequest,
    ) -> impl std::future::Future<Output = Result<(), ApplicationError>> {
        let span = tracing::info_span!("project_deletion", project_id = %request.project_id, action = "project_deletion");
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span.clone(),
            crate::observability::execution_summary::OperationSummary::ProjectDeletion {
                project_id: request.project_id.to_string(),
                action: "project_deletion",
                status: "aborted".to_string(),
                deleted_count: 0,
                failed_count: 0,
            },
        );

        let storage_uow = self.storage_uow.clone();
        let job_runtime = self.job_runtime.clone();
        let locks = self.locks.clone();

        async move {
            let project_id_str = request.project_id.to_string();
            let res = Self::execute_inner(request, storage_uow, job_runtime, locks).await;
            match &res {
                Ok((deleted_count, failed_count)) => {
                    let status = if *failed_count > 0 {
                        "completed_with_errors"
                    } else {
                        "completed"
                    };
                    guard.update_summary(crate::observability::execution_summary::OperationSummary::ProjectDeletion {
                        project_id: project_id_str,
                        action: "project_deletion",
                        status: status.to_string(),
                        deleted_count: *deleted_count as u64,
                        failed_count: *failed_count as u64,
                    });
                }
                Err(_) => guard.summary.update_status("failed"),
            }
            res.map(|_| ())
        }
    }

    async fn execute_inner(
        request: DeleteProjectRequest,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        job_runtime: Arc<dyn JobRuntimeControlPort>,
        locks: Arc<ProjectLifecycleLocks>,
    ) -> Result<(usize, usize), ApplicationError> {
        let project_id = &request.project_id;

        let lock_arc = locks.get_lock(project_id)?;
        let _lock = lock_arc.lock().await;

        // 1. Execute transaction
        let commit = CommitProjectDelete {
            project_id: project_id.clone(),
        };

        let result = storage_uow.commit_project_delete(commit).await?;
        let deleted_count = result.deleted_job_ids.len();
        let mut failed_count = 0;

        // 2. Perform runtime cleanup
        match job_runtime
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
                            ports::job_runtime_control::RuntimeCleanupOutcome::Unconfirmed
                        )
                    })
                    .map(|(id, _)| id.to_string())
                    .collect();

                if !unconfirmed.is_empty() {
                    failed_count += unconfirmed.len();
                    tracing::error!(
                        error = %common::observability::redaction::DiagnosticError {
                            kind: "ProjectDeletionUnconfirmedJobs",
                            code: None,
                            retryable: false,
                        },
                        project_id = %project_id,
                        unconfirmed_jobs = ?unconfirmed,
                        "CRITICAL: Failed to confirm termination of the following jobs after abort"
                    );
                }
            }
            Err(_e) => {
                failed_count += deleted_count; // Rough approximation if cleanup fails completely
                tracing::error!(
                    error = %common::observability::redaction::DiagnosticError {
                        kind: "ProjectDeletionRuntimeCleanupFailed",
                        code: None,
                        retryable: false,
                    },
                    project_id = %project_id,
                    "CRITICAL: Failed to cleanup runtime jobs for project"
                );
                // We do not return an error to the UI because the project is already persistently deleted.
            }
        }

        Ok((deleted_count, failed_count))
    }
}
