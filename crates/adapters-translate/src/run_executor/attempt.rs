use super::frozen_input::PreparedRun;
use super::{RunExecutionOptions, invalid, storage};
use auralis_translation::{
    ProgressSink, ResultId, RetryPolicy, ReviewState, RunProgress, RunState,
};
use auralis_translation_llamacpp::{
    LlamaCppProvider, RequestControlPolicy, verify_server_with_control,
};
use auralis_translation_sqlite::{ResultSpec, RunStop, SqliteConfig, TranslateDb};
use ports::committed_translation_progress::CommittedTranslationProgress;
use ports::error::PortError;
use ports::translation_run_control::TranslationRunPhase;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

pub(super) fn execute_attempt(
    database_path: &Path,
    prepared: PreparedRun,
    server_url: &str,
    options: RunExecutionOptions,
) -> Result<Uuid, PortError> {
    let PreparedRun {
        mut database,
        run,
        run_id,
        plan,
        profile,
        ..
    } = prepared;
    let start_guard = match options.start_guard {
        Some(guard) => {
            if guard.run_id != run_id.get() {
                return Err(PortError::Cancelled);
            }
            let state = match guard.phase {
                TranslationRunPhase::Requested => RunState::Requested,
                TranslationRunPhase::Paused => RunState::Paused,
                TranslationRunPhase::Failed => RunState::Failed,
                _ => return Err(invalid("invalid admission phase")),
            };
            auralis_translation_sqlite::AttemptStartGuard::new(
                run_id,
                state,
                guard.control_revision,
            )
            .map_err(|error| storage("read_admission_guard", error))?
        }
        None => database
            .capture_attempt_start(run_id)
            .map_err(|error| storage("capture_admission_guard", error))?,
    };
    let current_guard = database
        .capture_attempt_start(run_id)
        .map_err(|error| storage("check_admission_guard", error))?;
    if current_guard != start_guard {
        return Err(PortError::Cancelled);
    }
    let provider = LlamaCppProvider::new(server_url, profile.clone()).map_err(|error| {
        PortError::ExternalToolFailed {
            tool: "llama.cpp".into(),
            message: error.to_string(),
        }
    })?;
    let preparation = auralis_translation_sqlite::AttemptStartControl::new(
        &database,
        start_guard,
        RequestControlPolicy::default().poll_interval(),
    )
    .map_err(|error| storage("prepare_translation_control", error))?;
    let check = || {
        preparation
            .check()
            .map_err(|cause| auralis_translation::ProviderError::Permanent(cause.to_string()))
    };
    let verification = verify_server_with_control(&provider, &profile, &check);
    database
        .check_attempt_start(start_guard)
        .map_err(|error| match error {
            auralis_translation_sqlite::DbError::PauseRequested => PortError::Cancelled,
            other => storage("check_translation_preflight", other),
        })?;
    verification.map_err(|error| PortError::ExternalToolFailed {
        tool: "llama.cpp preflight".into(),
        message: error.to_string(),
    })?;
    let retry = RetryPolicy::new(profile.max_block_attempts)
        .ok_or_else(|| invalid("invalid model retry policy"))?;
    let control = TranslateDb::open(database_path, SqliteConfig::default())
        .map_err(|error| storage("open_translate_control", error))?;
    let attempt = database
        .begin_guarded_attempt(&run, options.host_job_id.as_deref(), start_guard)
        .map_err(|error| match error {
            auralis_translation_sqlite::DbError::PauseRequested => PortError::Cancelled,
            other => storage("begin_translate_attempt", other),
        })?;
    let mut reporter = ForwardProgress {
        sender: options.progress,
    };
    if let Err(error) =
        plan.execute_with_policy(&provider, &mut database, &mut reporter, &control, retry)
    {
        let (stop, reason) = if matches!(error, PortError::Cancelled) {
            (RunStop::Paused, "pause requested")
        } else {
            (RunStop::Failed, "Translate execution did not complete")
        };
        database
            .stop_attempt(run_id, attempt, stop, reason)
            .map_err(|stop_error| storage("stop_translate_attempt", stop_error))?;
        return Err(error);
    }
    let result_id = ResultId::new(Uuid::new_v4()).ok_or_else(|| invalid("result ID is nil"))?;
    let result = database.commit_result(
        &ResultSpec {
            result_id,
            run_id,
            revision: 1,
            source_hash: plan.source_hash(),
            block_fingerprints: plan.block_fingerprints(),
            review_state: ReviewState::NeedsReview,
        },
        &plan,
    );
    match result {
        Ok(record) => Ok(record.result_id.get()),
        Err(error) => {
            let stop = if matches!(error, auralis_translation_sqlite::DbError::PauseRequested) {
                RunStop::Paused
            } else {
                RunStop::Failed
            };
            database
                .stop_attempt(run_id, attempt, stop, "Translate result did not commit")
                .map_err(|stop_error| storage("stop_translate_attempt", stop_error))?;
            if matches!(error, auralis_translation_sqlite::DbError::PauseRequested) {
                Err(PortError::Cancelled)
            } else {
                Err(storage("commit_translate_result", error))
            }
        }
    }
}

struct ForwardProgress {
    sender: Option<UnboundedSender<CommittedTranslationProgress>>,
}

impl ProgressSink for ForwardProgress {
    fn report(&mut self, progress: RunProgress) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(CommittedTranslationProgress {
                run_id: progress.run_id.get(),
                saved_blocks: progress.committed_blocks,
                total_blocks: progress.total_blocks,
            });
        }
    }
}
