#![allow(clippy::unwrap_used, clippy::expect_used)]
use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::transaction::{CommitPipelineStart, CommitPipelineStartFailure, StorageUnitOfWork};
use std::sync::Arc;

use crate::error::ApplicationError;
use crate::usecases::pipeline::mock_dubbing_pipeline::MockDubbingPipelineRunner;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::workspace::TempWorkspacePort;

#[derive(Debug)]
pub struct StartMockPipelineRequest {
    pub project_id: ProjectId,
}

#[derive(Debug)]
pub struct StartMockPipelineResponse {
    pub project: Project,
    pub job: ScheduledJob,
}

pub struct StartMockPipelineUseCase<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    project_repo: R,
    job_scheduler: Arc<dyn JobSchedulerPort>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    subtitle_source: V,
    artifact_store: S,
    workspace_port: Arc<dyn TempWorkspacePort>,
    locks: Arc<crate::usecases::project::lifecycle::ProjectLifecycleLocks>,
    job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> StartMockPipelineUseCase<R, V, S>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_repo: R,
        job_scheduler: Arc<dyn JobSchedulerPort>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        subtitle_source: V,
        artifact_store: S,
        workspace_port: Arc<dyn TempWorkspacePort>,
        locks: Arc<crate::usecases::project::lifecycle::ProjectLifecycleLocks>,
        job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
    ) -> Self {
        Self {
            project_repo,
            job_scheduler,
            storage_uow,
            subtitle_source,
            artifact_store,
            workspace_port,
            locks,
            job_runtime,
        }
    }

    pub async fn execute(
        &self,
        request: StartMockPipelineRequest,
    ) -> Result<StartMockPipelineResponse, ApplicationError> {
        // 1. Acquire ProjectLifecycleLock
        let lock_arc = self.locks.get_lock(&request.project_id)?;
        let _lock = lock_arc.lock().await;

        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        // Create Job
        let job = domain::job::Job::new(
            project.id().clone(),
            project.title().to_string(),
            domain::job::JobKind::Dubbing,
        );
        let job_id = job.id().clone();
        project.start_processing(job_id.clone())?;

        // 2. Commit Pending job to DB
        let commit_cmd = CommitPipelineStart {
            project: project.clone(),
            job: job.clone(),
        };
        self.storage_uow
            .commit_pipeline_start(commit_cmd)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to commit pipeline start: {}", e),
            })?; // If failed -> no runtime registration

        // 3. job_runtime.reserve()
        if let Err(e) = self
            .job_runtime
            .reserve(job_id.clone(), request.project_id.clone())
            .await
        {
            // Apply compensation for DB job
            let _ = self
                .compensate_start_failure(&project, &job, e.to_string())
                .await;
            return Err(ApplicationError::InvalidOperation {
                message: format!("Failed to reserve runtime: {}", e),
            });
        }

        // 4. Create tokens, Completion, and Two-Phase Gates
        let (cancel_handle, token) = ports::cancellation::CancelHandle::new();
        let completion = Arc::new(ports::job_runtime_control::RuntimeCompletion::new());
        let (activate_tx, activate_rx) = tokio::sync::oneshot::channel::<()>();
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel::<()>();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();

        struct CompletionGuard {
            job_id: domain::job::JobId,
            completion: Arc<ports::job_runtime_control::RuntimeCompletion>,
            job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
        }
        impl Drop for CompletionGuard {
            fn drop(&mut self) {
                let mut outcome_guard = self
                    .completion
                    .outcome
                    .lock()
                    .unwrap_or_else(|p| p.into_inner());
                if outcome_guard.is_none() {
                    *outcome_guard =
                        Some(ports::job_runtime_control::RuntimeTaskOutcome::Cancelled);
                }
                self.completion
                    .state
                    .store(1, std::sync::atomic::Ordering::Release);
                self.completion.notify.notify_waiters();
                self.job_runtime.finish_now(&self.job_id);
            }
        }

        let runner = MockDubbingPipelineRunner::new(
            self.job_scheduler.clone(),
            self.project_repo.clone(),
            self.subtitle_source.clone(),
            self.storage_uow.clone(),
            self.artifact_store.clone(),
            self.workspace_port.clone(),
            self.job_runtime.clone(),
        );

        let job_id_clone = job_id.clone();
        let project_id_clone = request.project_id.clone();
        let completion_clone = completion.clone();
        let runtime_clone = self.job_runtime.clone();

        let job_scheduler_clone = self.job_scheduler.clone();

        let span = tracing::info_span!("job_execution", job_id = %job_id, project_id = %request.project_id, action = "job_execution");

        let span_clone_for_spawn = span.clone();

        // 5. Spawn the task wrapper
        let wrapper = async move {
            let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
                span.clone(),
                crate::observability::execution_summary::OperationSummary::JobExecution {
                    project_id: project_id_clone.to_string(),
                    job_id: job_id_clone.to_string(),
                    action: "job_execution",
                    status: "aborted".to_string(),
                },
            );

            let _c_guard = CompletionGuard {
                job_id: job_id_clone.clone(),
                completion: completion_clone,
                job_runtime: runtime_clone,
            };

            // Two-Phase Start Gate
            if activate_rx.await.is_err() {
                guard.summary.update_status("cancelled_at_activate");
                return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
            }
            if ack_tx.send(()).is_err() {
                guard.summary.update_status("cancelled_at_ack");
                return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
            }
            let mut release_fut = std::pin::pin!(release_rx);
            loop {
                if token.is_cancelled() {
                    guard.summary.update_status("cancelled_before_release");
                    return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
                }
                tokio::select! {
                    res = &mut release_fut => {
                        if res.is_err() {
                            guard.summary.update_status("cancelled_at_release");
                            return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
                        }
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
                }
            }

            let actual_run = async {
                runner
                    .run(job_id_clone.clone(), project_id_clone, token, &mut guard)
                    .await
            };

            let outcome =
                match futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(actual_run))
                    .await
                {
                    Ok(o) => o,
                    Err(_) => {
                        // Panic -> Terminal Failed
                        let _ = job_scheduler_clone
                            .fail_job(&job_id_clone, "PANIC".into(), "Task panicked".into(), false)
                            .await;
                        ports::job_runtime_control::RuntimeTaskOutcome::Panicked
                    }
                };

            let mut outcome_guard = _c_guard
                .completion
                .outcome
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            *outcome_guard = Some(outcome);
            outcome
        };

        // 6. Obtain JoinHandle, build RuntimeTask, attach
        let join_handle = tokio::spawn(tracing::Instrument::instrument(
            wrapper,
            span_clone_for_spawn,
        ));
        let runtime_task = ports::job_runtime_control::RuntimeTask {
            cancel: cancel_handle.clone(),
            join_handle,
            completion,
        };

        // 7. Attach Task
        if let Err(e) = self
            .job_runtime
            .attach_task(job_id.clone(), runtime_task)
            .await
        {
            // Take RuntimeTask back, abort, await, remove reservation, apply compensation
            e.task.cancel.cancel();
            e.task.join_handle.abort();
            let _ = e.task.join_handle.await;
            self.job_runtime.finish_now(&job_id); // remove reservation
            let _ = self
                .compensate_start_failure(&project, &job, e.source.to_string())
                .await;
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to attach task".into(),
            });
        }

        // 8. Atomic Commit
        let enqueue_result = self.job_scheduler.enqueue_existing_job(&job_id).await;
        let scheduled_job = match enqueue_result {
            Ok(scheduled) => scheduled,
            Err(enqueue_err) => {
                // rollback_runtime_start, abort, await, compensation
                let _ = self.job_runtime.rollback_runtime_start(&job_id).await;
                if let Err(comp_err) = self
                    .compensate_start_failure(&project, &job, enqueue_err.to_string())
                    .await
                {
                    return Err(ApplicationError::PipelineStartFailedNeedsRecovery {
                        scheduling_error: enqueue_err.to_string(),
                        compensation_error: comp_err.to_string(),
                    });
                }
                return Err(ApplicationError::PipelineStartFailed {
                    scheduling_error: enqueue_err.to_string(),
                });
            }
        };

        // 9. Send activate and await ack
        if activate_tx.send(()).is_err() {
            let _ = self.job_runtime.rollback_runtime_start(&job_id).await;
            let _ = self
                .compensate_start_failure(&project, &job, "Failed to activate".into())
                .await;
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to activate task".into(),
            });
        }

        if ack_rx.await.is_err() {
            let _ = self.job_runtime.rollback_runtime_start(&job_id).await;
            let _ = self
                .compensate_start_failure(&project, &job, "Failed to ack".into())
                .await;
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to ack task".into(),
            });
        }

        // Event publication
        // Currently JobManager publishes events on enqueue_existing_job.

        // Open start barrier
        if release_tx.send(()).is_err() {
            let _ = self.job_runtime.rollback_runtime_start(&job_id).await;
            let _ = self
                .compensate_start_failure(&project, &job, "Failed to release".into())
                .await;
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to release task".into(),
            });
        }

        Ok(StartMockPipelineResponse {
            project,
            job: scheduled_job,
        })
    }

    async fn compensate_start_failure(
        &self,
        project: &Project,
        job: &domain::job::Job,
        error_msg: String,
    ) -> Result<(), ApplicationError> {
        let mut failed_project = project.clone();
        let mut failed_job = job.clone();
        let expected_job_revision = failed_job.revision();

        failed_project
            .apply_terminal_transition(failed_job.id(), domain::job::TerminalOutcome::Failed)?;
        failed_job.mark_failed(domain::job::JobError::new(
            "SCHEDULING_FAILED",
            format!("Failed to schedule job: {}", error_msg),
            false,
        ))?;

        let failure_cmd = CommitPipelineStartFailure {
            project: failed_project,
            job: failed_job,
            expected_job_revision,
        };

        self.storage_uow
            .commit_pipeline_start_failure(failure_cmd)
            .await
            .map_err(|e| ApplicationError::PipelineStartFailedNeedsRecovery {
                scheduling_error: error_msg,
                compensation_error: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockJobScheduler, MockStorageUnitOfWork};
    use adapters_storage::memory::InMemoryProjectRepository;
    use async_trait::async_trait;
    use domain::job::JobStatus;

    use ports::error::PortError;

    #[derive(Clone)]
    struct MockSubtitleSource;

    #[async_trait]
    impl SubtitleSourcePort for MockSubtitleSource {
        async fn list_subtitles(
            &self,
            _source: &domain::media::MediaSource,
        ) -> Result<Vec<domain::media::SubtitleTrack>, PortError> {
            Ok(vec![])
        }

        async fn download_subtitle(
            &self,
            _request: ports::source::DownloadSubtitleRequest,
        ) -> Result<domain::media::Artifact, PortError> {
            Err(PortError::Unsupported {
                message: "Not implemented".into(),
            })
        }
    }

    use crate::test_utils::MockArtifactStore;
    use adapters_storage::local::LocalTempWorkspace;

    struct MockJobRuntimeControl;
    #[async_trait::async_trait]
    impl ports::job_runtime_control::JobRuntimeControlPort for MockJobRuntimeControl {
        async fn cancel_and_evict_jobs(
            &self,
            _job_ids: &[domain::job::JobId],
        ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, ports::error::PortError>
        {
            Ok(ports::job_runtime_control::RuntimeCleanupReport {
                jobs: std::collections::HashMap::new(),
            })
        }
        async fn reserve(
            &self,
            _job_id: domain::job::JobId,
            _project_id: domain::project::ProjectId,
        ) -> Result<(), ports::error::PortError> {
            Ok(())
        }
        async fn attach_task(
            &self,
            _job_id: domain::job::JobId,
            _task: ports::job_runtime_control::RuntimeTask,
        ) -> Result<(), ports::job_runtime_control::AttachTaskError> {
            Ok(())
        }
        fn finish_now(&self, _job_id: &domain::job::JobId) {}
        async fn rollback_runtime_start(
            &self,
            _job_id: &domain::job::JobId,
        ) -> Result<ports::job_runtime_control::RuntimeCleanupOutcome, ports::error::PortError>
        {
            Ok(ports::job_runtime_control::RuntimeCleanupOutcome::ReservationRemoved)
        }
    }

    #[tokio::test]
    async fn test_success_saves_project_processing_and_job_pending_running() {
        let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(
            std::sync::Mutex::new(adapters_storage::memory::InMemoryDatabase::new()),
        ));
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockStorageUnitOfWork::new());

        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
            Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
            Arc::new(MockJobRuntimeControl),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.job.status, JobStatus::Running); // MockJobScheduler sets to Running

        // Verify project in the database via the mock uow directly,
        // since the in-memory adapter saves via repository
        let projects_saved = tx_gateway.projects_saved.lock().await;
        assert_eq!(projects_saved.len(), 1);
        assert_eq!(
            projects_saved[0].status(),
            &domain::project::ProjectStatus::Processing
        );

        let jobs_saved = tx_gateway.jobs_saved.lock().await;
        assert_eq!(jobs_saved.len(), 1);
        assert_eq!(jobs_saved[0].status(), &JobStatus::Pending); // The usecase saves as pending!
    }

    #[tokio::test]
    async fn test_transaction_failure_does_not_enqueue_job() {
        let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(
            std::sync::Mutex::new(adapters_storage::memory::InMemoryDatabase::new()),
        ));
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockStorageUnitOfWork::with_failure()); // Will fail

        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
            Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
            Arc::new(MockJobRuntimeControl),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(
            response,
            Err(ApplicationError::InvalidOperation { .. })
        ));

        // Verify job was NOT enqueued
        let scheduled_jobs = job_scheduler.jobs.lock().await;
        assert_eq!(scheduled_jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_cannot_start_from_draft() {
        let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(
            std::sync::Mutex::new(adapters_storage::memory::InMemoryDatabase::new()),
        ));
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockStorageUnitOfWork::new());

        let project = Project::new("Test".to_string());
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
            Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
            Arc::new(MockJobRuntimeControl),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(response, Err(ApplicationError::Domain { .. })));
    }

    #[tokio::test]
    async fn test_cannot_start_from_completed() {
        let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(
            std::sync::Mutex::new(adapters_storage::memory::InMemoryDatabase::new()),
        ));
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockStorageUnitOfWork::new());

        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        let test_job_id = domain::job::JobId::new();
        project.start_processing(test_job_id.clone()).unwrap();
        project
            .apply_terminal_transition(&test_job_id, domain::job::TerminalOutcome::Completed)
            .unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
            Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
            Arc::new(MockJobRuntimeControl),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(response, Err(ApplicationError::Domain { .. })));
    }

    #[tokio::test]
    async fn test_enqueue_failure_compensates_and_marks_failed() {
        let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(
            std::sync::Mutex::new(adapters_storage::memory::InMemoryDatabase::new()),
        ));
        let mut job_scheduler = MockJobScheduler::new();
        job_scheduler.should_fail = true; // Make enqueue fail
        let job_scheduler = Arc::new(job_scheduler);
        let tx_gateway = Arc::new(MockStorageUnitOfWork::new());

        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
            Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
            Arc::new(MockJobRuntimeControl),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;

        match response {
            Err(ApplicationError::PipelineStartFailed { scheduling_error }) => {
                assert!(
                    scheduling_error.contains("queue is full")
                        || scheduling_error.contains("Simulated")
                        || !scheduling_error.is_empty()
                );
            }
            _ => panic!("Expected enqueue failure error"),
        }

        // Verify compensation occurred
        let projects_saved = tx_gateway.projects_saved.lock().await;
        // 2 saves: initial start, and compensation
        assert_eq!(projects_saved.len(), 2);
        assert_eq!(
            projects_saved[1].status(),
            &domain::project::ProjectStatus::Failed
        );

        let jobs_saved = tx_gateway.jobs_saved.lock().await;
        assert_eq!(jobs_saved.len(), 2);
        assert_eq!(jobs_saved[1].status(), &JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_enqueue_and_compensation_failure_returns_both_errors() {
        let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(
            std::sync::Mutex::new(adapters_storage::memory::InMemoryDatabase::new()),
        ));
        let mut job_scheduler = MockJobScheduler::new();
        job_scheduler.should_fail = true; // Make enqueue fail
        let job_scheduler = Arc::new(job_scheduler);

        // Custom mock to fail only on compensation
        #[derive(Clone)]
        struct FailCompUow {
            inner: Arc<MockStorageUnitOfWork>,
        }
        #[async_trait]
        impl StorageUnitOfWork for FailCompUow {
            async fn commit_artifact_finalize(
                &self,
                cmd: ports::transaction::CommitArtifactFinalize,
            ) -> Result<ports::transaction::CommitArtifactFinalizeResult, PortError> {
                self.inner.commit_artifact_finalize(cmd).await
            }
            async fn commit_transcript_import(
                &self,
                cmd: ports::transaction::CommitTranscriptImport,
            ) -> Result<(), PortError> {
                self.inner.commit_transcript_import(cmd).await
            }
            async fn commit_staged_artifact_write(
                &self,
                cmd: ports::transaction::CommitStagedArtifactWrite,
            ) -> Result<(), PortError> {
                self.inner.commit_staged_artifact_write(cmd).await
            }

            async fn commit_managed_source_import(
                &self,
                cmd: ports::transaction::CommitManagedSourceImport,
            ) -> Result<(), PortError> {
                self.inner.commit_managed_source_import(cmd).await
            }
            async fn commit_project_delete(
                &self,
                cmd: ports::transaction::CommitProjectDelete,
            ) -> Result<ports::transaction::CommitProjectDeleteResult, PortError> {
                self.inner.commit_project_delete(cmd).await
            }
            async fn commit_job_update(
                &self,
                cmd: ports::transaction::CommitJobUpdate,
            ) -> Result<(), PortError> {
                self.inner.commit_job_update(cmd).await
            }
            async fn commit_pipeline_start(
                &self,
                cmd: ports::transaction::CommitPipelineStart,
            ) -> Result<(), PortError> {
                self.inner.commit_pipeline_start(cmd).await
            }
            async fn commit_pipeline_start_failure(
                &self,
                _command: ports::transaction::CommitPipelineStartFailure,
            ) -> Result<(), PortError> {
                Err(PortError::Unexpected {
                    message: "UoW failed".into(),
                })
            }

            async fn commit_terminal_job_update(
                &self,
                _command: ports::transaction::CommitTerminalJobUpdate,
            ) -> Result<(), PortError> {
                Ok(())
            }

            async fn apply_terminal_lifecycle_conditionally(
                &self,
                _command: ports::transaction::ApplyTerminalLifecycle,
            ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
                Ok(domain::project::status::TerminalTransitionResult::Applied {
                    transcript_ready: false,
                })
            }
        }

        let tx_gateway = Arc::new(FailCompUow {
            inner: Arc::new(MockStorageUnitOfWork::new()),
        });

        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
            Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
            Arc::new(MockJobRuntimeControl),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;

        match response {
            Err(ApplicationError::PipelineStartFailedNeedsRecovery {
                scheduling_error,
                compensation_error,
            }) => {
                assert!(!scheduling_error.is_empty());
                assert!(compensation_error.contains("UoW failed"));
            }
            _ => panic!("Expected enqueue and compensation failure error"),
        }
    }
}
