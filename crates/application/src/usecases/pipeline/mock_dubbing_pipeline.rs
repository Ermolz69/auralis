use super::runner_terminalization::{
    TerminalFailure, await_or_cancel, cancelled, terminalize_runner_failure,
};
use crate::usecases::transcript::import_youtube_subtitles::{
    ImportYoutubeSubtitlesRequest, ImportYoutubeSubtitlesUseCase,
};
use domain::dubbing::DubbingPipelineStage;
use domain::job::{JobId, JobProgress};
use domain::project::ProjectId;
use ports::cancellation::CancellationToken;
use ports::error::PortError;
use ports::job_runtime_control::RuntimeTaskOutcome;
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
    #[cfg(test)]
    panic_on_run: bool,
}

#[cfg(test)]
#[path = "mock_dubbing_pipeline_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mock_dubbing_completion_tests.rs"]
mod completion_tests;

#[cfg(test)]
#[path = "mock_dubbing_panic_tests.rs"]
mod panic_tests;

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
            #[cfg(test)]
            panic_on_run: false,
        }
    }

    #[cfg(test)]
    pub fn with_panic_on_run(mut self) -> Self {
        self.panic_on_run = true;
        self
    }

    pub async fn run(
        &self,
        job_id: JobId,
        project_id: ProjectId,
        token: CancellationToken,
        guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
    ) -> RuntimeTaskOutcome {
        #[cfg(test)]
        if self.panic_on_run {
            panic!("test runner panic");
        }

        let stages = vec![
            (DubbingPipelineStage::ValidateSource, 10, 500),
            (DubbingPipelineStage::FetchMetadata, 25, 600),
            (DubbingPipelineStage::DownloadMedia, 45, 800),
        ];

        for (stage, percent, delay_ms) in stages {
            if token.is_cancelled() {
                return cancelled(guard);
            }

            let progress = JobProgress {
                percent: percent as u8,
                message: format!("Mock stage: {:?}", stage),
                current_step: Some(format!("{:?}", stage)),
                processed_items: None,
                total_items: None,
            };

            if let Err(outcome) = self
                .update_stage_or_terminalize(
                    &job_id,
                    &token,
                    guard,
                    stage,
                    progress,
                    TerminalFailure::stage_update(),
                )
                .await
            {
                return outcome;
            }

            if let Err(outcome) =
                await_or_cancel(&token, tokio::time::sleep(Duration::from_millis(delay_ms))).await
            {
                guard.summary.update_status("cancelled");
                return outcome;
            }
        }

        if let Err(outcome) = self
            .update_stage_or_terminalize(
                &job_id,
                &token,
                guard,
                DubbingPipelineStage::ExtractOrGenerateTranscript,
                JobProgress {
                    percent: 50,
                    message: "Extracting audio...".into(),
                    current_step: Some("extracting_audio".into()),
                    processed_items: None,
                    total_items: None,
                },
                TerminalFailure::stage_update(),
            )
            .await
        {
            return outcome;
        }

        let import_use_case = ImportYoutubeSubtitlesUseCase::new(
            Arc::new(self.project_repo.clone()),
            Arc::new(self.subtitle_source.clone()),
            Arc::new(self.artifact_store.clone()),
            Arc::new(self.storage_uow.clone()),
            self.workspace_port.clone(),
        );

        let tokio_token = tokio_util::sync::CancellationToken::new();
        let tokio_token_clone = tokio_token.clone();
        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = token_clone.cancelled() => {
                    tokio_token_clone.cancel();
                }
                _ = tokio_token_clone.cancelled() => {}
            }
        });

        match import_use_case
            .execute(ImportYoutubeSubtitlesRequest {
                project_id: project_id.clone(),
                preferred_languages: vec!["en".to_string(), "ru".to_string(), "uk".to_string()],
                allow_auto_generated: true,
                cancellation_token: tokio_token,
                job_id: job_id.clone(),
            })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                if token.is_cancelled() {
                    let is_cleanup_fail = match &e {
                        crate::error::ApplicationError::OperationFailedWithCleanup {
                            cleanup_report,
                            ..
                        } => !cleanup_report.failures.is_empty(),
                        _ => false,
                    };
                    if is_cleanup_fail {
                        guard.summary.update_status("cleanup_failed");
                        return RuntimeTaskOutcome::RecoveryRequired;
                    }
                    guard.summary.update_status("cancelled");
                    return RuntimeTaskOutcome::Cancelled;
                }
                return terminalize_runner_failure(
                    self.job_scheduler.as_ref(),
                    &job_id,
                    TerminalFailure::subtitle_import(),
                    guard,
                )
                .await;
            }
        }

        if token.is_cancelled() {
            return cancelled(guard);
        }

        if let Err(outcome) = self
            .update_stage_or_terminalize(
                &job_id,
                &token,
                guard,
                DubbingPipelineStage::ExportResult,
                JobProgress {
                    percent: 100,
                    message: "Mock stage: ExportResult".into(),
                    current_step: Some("export_result".into()),
                    processed_items: None,
                    total_items: None,
                },
                TerminalFailure::stage_update(),
            )
            .await
        {
            return outcome;
        }

        if let Err(outcome) =
            await_or_cancel(&token, tokio::time::sleep(Duration::from_millis(500))).await
        {
            guard.summary.update_status("cancelled");
            return outcome;
        }

        match await_or_cancel(&token, self.job_scheduler.complete_job(&job_id)).await {
            Ok(Ok(_)) => {
                guard.summary.update_status("completed");
                RuntimeTaskOutcome::Completed
            }
            Ok(Err(PortError::NotFound { .. })) => {
                guard.summary.update_status("deleted");
                RuntimeTaskOutcome::DeletedNoOp
            }
            Ok(Err(_)) => {
                guard.summary.update_status("completion_recovery_required");
                RuntimeTaskOutcome::RecoveryRequired
            }
            Err(outcome) => outcome,
        }
    }

    async fn update_stage_or_terminalize(
        &self,
        job_id: &JobId,
        token: &CancellationToken,
        guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
        stage: DubbingPipelineStage,
        progress: JobProgress,
        failure: TerminalFailure,
    ) -> Result<(), RuntimeTaskOutcome> {
        match await_or_cancel(
            token,
            self.job_scheduler.update_job_stage(job_id, stage, progress),
        )
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(PortError::NotFound { .. })) => {
                guard.summary.update_status("deleted");
                Err(RuntimeTaskOutcome::DeletedNoOp)
            }
            Ok(Err(_)) => {
                Err(
                    terminalize_runner_failure(self.job_scheduler.as_ref(), job_id, failure, guard)
                        .await,
                )
            }
            Err(outcome) => {
                guard.summary.update_status("cancelled");
                Err(outcome)
            }
        }
    }
}
