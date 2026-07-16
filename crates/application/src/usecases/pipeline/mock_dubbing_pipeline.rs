use crate::usecases::transcript::import_youtube_subtitles::{
    ImportYoutubeSubtitlesRequest, ImportYoutubeSubtitlesUseCase,
};
use domain::dubbing::DubbingPipelineStage;
use domain::job::{JobId, JobProgress, JobStatus};
use domain::project::ProjectId;
use ports::job_scheduler::JobSchedulerPort;
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

pub struct MockDubbingPipelineRunner<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    T: StorageUnitOfWork + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    job_scheduler: Arc<dyn JobSchedulerPort>,
    project_repo: R,
    subtitle_source: V,
    storage_uow: T,
    artifact_store: S,
    workspace_port: Arc<dyn TempWorkspacePort>,
    job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    T: StorageUnitOfWork + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> MockDubbingPipelineRunner<R, V, T, S>
{
    pub fn new(
        job_scheduler: Arc<dyn JobSchedulerPort>,
        project_repo: R,
        subtitle_source: V,
        storage_uow: T,
        artifact_store: S,
        workspace_port: Arc<dyn TempWorkspacePort>,
        job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
    ) -> Self {
        Self {
            job_scheduler,
            project_repo,
            subtitle_source,
            storage_uow,
            artifact_store,
            workspace_port,
            job_runtime,
        }
    }

    pub fn spawn(self, job_id: JobId, project_id: ProjectId) {
        let (cancel_handle, token) = ports::cancellation::CancelHandle::new();
        let (state_tx, state_rx) =
            tokio::sync::watch::channel(ports::job_runtime_control::RuntimeState::Starting);

        let runtime_clone = self.job_runtime.clone();
        let job_id_clone = job_id.clone();
        let project_id_clone = project_id.clone();

        let span = tracing::info_span!("job_execution", job_id = %job_id, project_id = %project_id, action = "job_execution");
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span.clone(),
            crate::observability::execution_summary::OperationSummary::JobExecution {
                project_id: project_id.to_string(),
                job_id: job_id.to_string(),
                action: "job_execution",
                status: "aborted".to_string(),
            },
        );

        let runner = async move {
            let _ = state_tx.send(ports::job_runtime_control::RuntimeState::Running);
            self.run(job_id_clone, project_id_clone, token, &mut guard)
                .await;
            let _ = state_tx.send(ports::job_runtime_control::RuntimeState::Finished);
        };

        let join_handle = tokio::spawn(tracing::Instrument::instrument(runner, span));
        let abort_handle = join_handle.abort_handle();

        // Register the task synchronously by spawning another short-lived task
        tokio::spawn(async move {
            runtime_clone
                .register_runtime_task(job_id, cancel_handle, state_rx, abort_handle)
                .await;
        });
    }

    #[allow(clippy::collapsible_if)]
    async fn run(
        &self,
        job_id: JobId,
        project_id: ProjectId,
        token: ports::cancellation::CancellationToken,
        guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
    ) {
        let stages = vec![
            (DubbingPipelineStage::ValidateSource, 10, 500),
            (DubbingPipelineStage::FetchMetadata, 25, 600),
            (DubbingPipelineStage::DownloadMedia, 45, 800),
        ];

        for (stage, percent, delay_ms) in stages {
            if token.is_cancelled() {
                guard.summary.update_status("cancelled");
                return;
            }

            match self.job_scheduler.get_job(&job_id).await {
                Ok(Some(job)) => {
                    if job.status == JobStatus::Cancelled {
                        guard.summary.update_status("cancelled");
                        return;
                    }
                }
                _ => {
                    guard.summary.update_status("deleted");
                    return;
                }
            }

            let progress = JobProgress {
                percent: percent as u8,
                message: format!("Mock stage: {:?}", stage),
                current_step: Some(format!("{:?}", stage)),
                processed_items: None,
                total_items: None,
            };

            let _ = self
                .job_scheduler
                .update_job_stage(&job_id, stage.clone(), progress)
                .await;

            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }

        if token.is_cancelled() {
            guard.summary.update_status("cancelled");
            return;
        }

        // ExtractOrGenerateTranscript stage (Real work: Subtitles Import)
        match self.job_scheduler.get_job(&job_id).await {
            Ok(Some(job)) => {
                if job.status == JobStatus::Cancelled {
                    guard.summary.update_status("cancelled");
                    return;
                }
            }
            _ => {
                guard.summary.update_status("deleted");
                return;
            }
        }

        let _ = self
            .job_scheduler
            .update_job_stage(
                &job_id,
                DubbingPipelineStage::ExtractOrGenerateTranscript,
                JobProgress {
                    percent: 50,
                    message: "Extracting audio...".into(),
                    current_step: Some("extracting_audio".into()),
                    processed_items: None,
                    total_items: None,
                },
            )
            .await;

        let import_use_case = ImportYoutubeSubtitlesUseCase::new(
            Arc::new(self.project_repo.clone()),
            Arc::new(self.subtitle_source.clone()),
            Arc::new(self.artifact_store.clone()),
            Arc::new(self.storage_uow.clone()),
            self.workspace_port.clone(),
        );

        match import_use_case
            .execute(ImportYoutubeSubtitlesRequest {
                project_id: project_id.clone(),
                preferred_languages: vec!["en".to_string(), "ru".to_string(), "uk".to_string()],
                allow_auto_generated: true,
            })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                let _ = self
                    .job_scheduler
                    .fail_job(
                        &job_id,
                        "SUBTITLE_IMPORT_FAILED".into(),
                        e.to_string(),
                        false,
                    )
                    .await;
                guard.summary.update_status("failed");
                return;
            }
        }

        // Check cancellation again
        match self.job_scheduler.get_job(&job_id).await {
            Ok(Some(job)) => {
                if job.status == JobStatus::Cancelled {
                    guard.summary.update_status("cancelled");
                    return;
                }
            }
            _ => {
                guard.summary.update_status("deleted");
                return;
            }
        }

        // ExportResult stage
        let _ = self
            .job_scheduler
            .update_job_stage(
                &job_id,
                DubbingPipelineStage::ExportResult,
                JobProgress {
                    percent: 100,
                    message: "Mock stage: ExportResult".into(),
                    current_step: Some("export_result".into()),
                    processed_items: None,
                    total_items: None,
                },
            )
            .await;
        sleep(Duration::from_millis(500)).await;

        let _ = self.job_scheduler.complete_job(&job_id).await;
        guard.summary.update_status("completed");
    }
}
