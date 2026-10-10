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
    selected_subtitle_track: Option<domain::media::SubtitleTrack>,
    execution_inputs: Option<ports::job_execution::ExecutionDescriptor>,
    cpu_executor: Option<Arc<dyn ports::job_resources::CpuExecutorPort>>,
    max_subtitle_bytes: u64,
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
            selected_subtitle_track: None,
            execution_inputs: None,
            cpu_executor: None,
            max_subtitle_bytes: 10 * 1024 * 1024,
            #[cfg(test)]
            panic_on_run: false,
        }
    }

    pub fn with_selected_subtitle_track(
        mut self,
        track: Option<domain::media::SubtitleTrack>,
    ) -> Self {
        self.selected_subtitle_track = track;
        self
    }

    pub(super) fn with_execution_inputs(
        mut self,
        inputs: ports::job_execution::ExecutionDescriptor,
        cpu: Arc<dyn ports::job_resources::CpuExecutorPort>,
        max_subtitle_bytes: u64,
    ) -> Self {
        self.selected_subtitle_track = inputs.selected_subtitle_track.clone();
        self.execution_inputs = Some(inputs);
        self.cpu_executor = Some(cpu);
        self.max_subtitle_bytes = max_subtitle_bytes;
        self
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

        if let Err(outcome) = self
            .update_stage_or_terminalize(
                &job_id,
                &token,
                guard,
                DubbingPipelineStage::ExtractOrGenerateTranscript,
                JobProgress {
                    percent: 10,
                    message: "Importing YouTube subtitles...".into(),
                    current_step: Some("importing_youtube_subtitles".into()),
                    processed_items: None,
                    total_items: None,
                },
                TerminalFailure::stage_update(),
            )
            .await
        {
            return outcome;
        }

        let mut import_use_case = ImportYoutubeSubtitlesUseCase::new(
            Arc::new(self.project_repo.clone()),
            Arc::new(self.subtitle_source.clone()),
            Arc::new(self.artifact_store.clone()),
            Arc::new(self.storage_uow.clone()),
            self.workspace_port.clone(),
        );
        if let Some(cpu) = &self.cpu_executor {
            import_use_case =
                import_use_case.with_cpu_executor(cpu.clone(), self.max_subtitle_bytes);
        }

        match import_use_case
            .execute(ImportYoutubeSubtitlesRequest {
                project_id: project_id.clone(),
                preferred_languages: self.execution_inputs.as_ref().map_or_else(
                    || vec!["ru".into(), "en".into(), "uk".into()],
                    |inputs| inputs.preferred_languages.clone(),
                ),
                allow_auto_generated: self
                    .execution_inputs
                    .as_ref()
                    .is_none_or(|inputs| inputs.allow_auto_generated),
                cancellation_token: token.clone(),
                job_id: job_id.clone(),
                selected_track: self.selected_subtitle_track.clone(),
            })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                if matches!(&e, crate::error::ApplicationError::OperationFailedWithCleanup { cleanup_report, .. } if !cleanup_report.is_empty())
                    || matches!(
                        &e,
                        crate::error::ApplicationError::Port(PortError::CleanupFailed { .. })
                    )
                {
                    guard.summary.update_status("cleanup_unconfirmed");
                    return RuntimeTaskOutcome::CleanupUnconfirmed;
                }
                if token.is_cancelled() {
                    let is_cleanup_fail = match &e {
                        crate::error::ApplicationError::OperationFailedWithCleanup {
                            cleanup_report,
                            ..
                        } => !cleanup_report.is_empty(),
                        _ => false,
                    };
                    if is_cleanup_fail {
                        guard.summary.update_status("cleanup_failed");
                        return RuntimeTaskOutcome::CleanupUnconfirmed;
                    }
                    guard.summary.update_status("cancelled");
                    return RuntimeTaskOutcome::Cancelled;
                }
                let rate_limited = matches!(
                    &e,
                    crate::error::ApplicationError::Port(
                        PortError::ExternalToolFailed { message, .. },
                    ) if message == "YouTube rate limit reached; try again later"
                );
                let failure = if rate_limited {
                    TerminalFailure::youtube_rate_limited()
                } else {
                    TerminalFailure::subtitle_import()
                };
                tracing::error!(
                    error = %common::observability::redaction::DiagnosticError {
                        kind: "subtitle_import",
                        code: Some(if rate_limited {
                            "YOUTUBE_RATE_LIMITED"
                        } else {
                            "SUBTITLE_IMPORT_FAILED"
                        }),
                        retryable: true,
                    },
                    "subtitle import failed"
                );
                return terminalize_runner_failure(
                    self.job_scheduler.as_ref(),
                    &job_id,
                    failure,
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
                DubbingPipelineStage::ExtractOrGenerateTranscript,
                JobProgress {
                    percent: 100,
                    message: "YouTube subtitles imported".into(),
                    current_step: Some("youtube_subtitles_imported".into()),
                    processed_items: None,
                    total_items: None,
                },
                TerminalFailure::stage_update(),
            )
            .await
        {
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
