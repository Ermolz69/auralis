use crate::usecases::transcript::import_youtube_subtitles::{
    ImportYoutubeSubtitlesRequest, ImportYoutubeSubtitlesUseCase,
};
use domain::dubbing::DubbingPipelineStage;
use domain::job::{JobId, JobProgress};
use domain::project::ProjectId;
use ports::job_scheduler::JobSchedulerPort;
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;
use std::sync::Arc;
use tokio::time::Duration;

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
    _job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
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
            _job_runtime: job_runtime,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub async fn run(
        &self,
        job_id: JobId,
        project_id: ProjectId,
        token: ports::cancellation::CancellationToken,
        guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
    ) -> ports::job_runtime_control::RuntimeTaskOutcome {
        let stages = vec![
            (DubbingPipelineStage::ValidateSource, 10, 500),
            (DubbingPipelineStage::FetchMetadata, 25, 600),
            (DubbingPipelineStage::DownloadMedia, 45, 800),
        ];

        for (stage, percent, delay_ms) in stages {
            if token.is_cancelled() {
                guard.summary.update_status("cancelled");
                return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
            }

            let progress = JobProgress {
                percent: percent as u8,
                message: format!("Mock stage: {:?}", stage),
                current_step: Some(format!("{:?}", stage)),
                processed_items: None,
                total_items: None,
            };

            if let Err(e) = self
                .job_scheduler
                .update_job_stage(&job_id, stage.clone(), progress)
                .await
            {
                match e {
                    ports::error::PortError::NotFound { .. } => {
                        guard.summary.update_status("deleted");
                        return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
                    }
                    ports::error::PortError::Conflict { .. } => {
                        guard.summary.update_status("conflict");
                        return ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed;
                    }
                    _ => {
                        // Storage / other
                        guard.summary.update_status("storage_error");
                        let _ = self
                            .job_scheduler
                            .fail_job(&job_id, "STORAGE_ERROR".into(), e.to_string(), false)
                            .await;
                        return ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed;
                    }
                }
            }

            // Before I/O check
            if token.is_cancelled() {
                guard.summary.update_status("cancelled");
                return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
            }

            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

            // After I/O check
            if token.is_cancelled() {
                guard.summary.update_status("cancelled");
                return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
            }
        }

        if let Err(e) = self
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
            .await
        {
            match e {
                ports::error::PortError::NotFound { .. } => {
                    guard.summary.update_status("deleted");
                    return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
                }
                _ => return ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed,
            }
        }

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
                return ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed;
            }
        }

        if token.is_cancelled() {
            guard.summary.update_status("cancelled");
            return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
        }

        if let Err(e) = self
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
            .await
        {
            match e {
                ports::error::PortError::NotFound { .. } => {
                    return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
                }
                _ => return ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed,
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        if let Err(e) = self.job_scheduler.complete_job(&job_id).await {
            match e {
                ports::error::PortError::NotFound { .. } => {
                    return ports::job_runtime_control::RuntimeTaskOutcome::Cancelled;
                }
                _ => return ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed,
            }
        }

        guard.summary.update_status("completed");
        ports::job_runtime_control::RuntimeTaskOutcome::Completed
    }
}
