use crate::format_inspector::read_bounded;
use crate::run_plan::DocumentPlan;
use async_trait::async_trait;
use auralis_translation::{
    BlockPolicy, LanguageCode, LanguagePair, ProgressSink, ResultId, RetryPolicy, ReviewState,
    RunId, RunProgress, RunState, SourceHash, TranslationId,
};
use auralis_translation_formats::srt::SrtParsePolicy;
use auralis_translation_formats::vtt::VttParsePolicy;
use auralis_translation_llamacpp::{
    LlamaCppProvider, ModelProfile, RequestControlPolicy, verify_server_with_control,
};
use auralis_translation_sqlite::{ResultSpec, RunStop, SqliteConfig, TranslateDb};
use ports::committed_translation_progress::CommittedTranslationProgress;
use ports::error::PortError;
use ports::host_translation_execution::HostTranslationExecution;
use ports::translation_execution::TranslationExecutionPort;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
use ports::translation_run_control::TranslationRunPhase;
use ports::translation_start_guard::TranslationStartGuard;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

pub struct TranslateRunExecutor {
    database_path: PathBuf,
    allow_unchecked_profile: bool,
}

impl TranslateRunExecutor {
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            database_path,
            allow_unchecked_profile: false,
        }
    }

    pub fn experimental_unchecked(database_path: PathBuf) -> Self {
        Self {
            database_path,
            allow_unchecked_profile: true,
        }
    }

    async fn run(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        server_url: &str,
        host: Option<HostTranslationExecution>,
    ) -> Result<Uuid, PortError> {
        let database_path = self.database_path.clone();
        let source_path = source_path.to_path_buf();
        let link = link.clone();
        let intent = intent.clone();
        let server_url = server_url.to_owned();
        let (host_job_id, start_guard, progress) = match host {
            Some(host) => (
                Some(host.job_id.to_string()),
                host.start_guard,
                Some(host.progress),
            ),
            None => (None, None, None),
        };
        let options = RunExecutionOptions {
            allow_unchecked_profile: self.allow_unchecked_profile,
            host_job_id,
            start_guard,
            progress,
        };
        tokio::task::spawn_blocking(move || {
            execute_run(
                &database_path,
                &source_path,
                &link,
                &intent,
                &server_url,
                options,
            )
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
}

#[async_trait]
impl TranslationExecutionPort for TranslateRunExecutor {
    async fn execute(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        server_url: &str,
    ) -> Result<Uuid, PortError> {
        self.run(source_path, link, intent, server_url, None).await
    }

    async fn execute_for_host(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        server_url: &str,
        host: HostTranslationExecution,
    ) -> Result<Uuid, PortError> {
        self.run(source_path, link, intent, server_url, Some(host))
            .await
    }
}

struct RunExecutionOptions {
    start_guard: Option<TranslationStartGuard>,
    allow_unchecked_profile: bool,
    host_job_id: Option<String>,
    progress: Option<UnboundedSender<CommittedTranslationProgress>>,
}

fn execute_run(
    database_path: &Path,
    source_path: &Path,
    link: &TranslationLink,
    intent: &TranslationRunIntent,
    server_url: &str,
    options: RunExecutionOptions,
) -> Result<Uuid, PortError> {
    if link.translation_id != intent.translation_id || link.active_run_id != Some(intent.run_id) {
        return Err(conflict("active link and frozen run intent differ"));
    }
    let translation_id =
        TranslationId::new(link.translation_id).ok_or_else(|| invalid("translation ID is nil"))?;
    let run_id = RunId::new(intent.run_id).ok_or_else(|| invalid("run ID is nil"))?;
    let pair = match (link.source_language.as_str(), link.target_language.as_str()) {
        ("zh", "ru") => LanguagePair::new(LanguageCode::Chinese, LanguageCode::Russian),
        ("ja", "ru") => LanguagePair::new(LanguageCode::Japanese, LanguageCode::Russian),
        _ => return Err(invalid("unsupported language pair")),
    }
    .map_err(|error| invalid(&error.to_string()))?;
    let profile_bytes = intent.profile_manifest_json.as_bytes();
    if SourceHash::digest(profile_bytes).to_string() != intent.profile_sha256 {
        return Err(conflict("frozen model profile hash differs from its bytes"));
    }
    let profile =
        ModelProfile::from_json(profile_bytes).map_err(|error| invalid(&error.to_string()))?;
    if profile.model_file_bytes.is_none() && !options.allow_unchecked_profile {
        return Err(PortError::Unsupported {
            message: "Auralis execution requires a checked model profile".into(),
        });
    }
    let policy = BlockPolicy::with_context(
        intent.max_target_segments as usize,
        intent.context_before_segments as usize,
        intent.context_after_segments as usize,
    )
    .ok_or_else(|| invalid("invalid block policy"))?;
    if profile.target_segments_per_block != policy.max_target_segments()
        || profile.context_before_segments != policy.context_before_segments()
        || profile.context_after_segments != policy.context_after_segments()
    {
        return Err(conflict(
            "profile block policy differs from frozen run intent",
        ));
    }
    let max_bytes = match link.source_format.as_str() {
        "srt" => SrtParsePolicy::default().max_bytes(),
        "vtt" => VttParsePolicy::default().max_bytes(),
        _ => return Err(invalid("unsupported subtitle format")),
    };
    let source = read_bounded(source_path, max_bytes)?;
    let plan = DocumentPlan::new(
        &link.source_format,
        &source,
        translation_id,
        run_id,
        pair,
        policy,
    )?;
    if plan.source_hash().to_string() != link.source_sha256
        || plan.parser_version() != intent.parser_version
        || plan.policy_fingerprint(policy).to_string() != intent.policy_fingerprint
    {
        return Err(conflict(
            "original or parser policy differs from frozen run",
        ));
    }
    let mut database = TranslateDb::open(database_path, SqliteConfig::default())
        .map_err(|error| storage("open_translate_database", error))?;
    let translation = database
        .translation(translation_id)
        .map_err(|error| storage("read_translate_translation", error))?;
    if translation.project_id.as_deref() != Some(link.project_id.to_string().as_str())
        || translation.source_artifact_id.as_deref()
            != Some(link.source_artifact_id.to_string().as_str())
        || translation.source_hash != plan.source_hash()
        || translation.source_format != link.source_format
    {
        return Err(conflict(
            "stored Translate source differs from project link",
        ));
    }
    let run = database
        .run(run_id)
        .map_err(|error| storage("read_translate_run", error))?;
    if run.translation_id != translation_id
        || run.source_hash != plan.source_hash()
        || run.profile_fingerprint != intent.profile_sha256
        || run.parser_version != intent.parser_version
        || run.policy_fingerprint != intent.policy_fingerprint
        || run.glossary_revision.is_some()
        || run.blocks != plan.blocks()
    {
        return Err(conflict(
            "stored Translate run differs from frozen Auralis intent",
        ));
    }
    match database
        .run_state(run_id)
        .map_err(|error| storage("read_translate_run_state", error))?
    {
        RunState::Validated => {
            let result = database
                .result_for_run(run_id)
                .map_err(|error| storage("read_translate_result", error))?;
            if result.run_id != run_id || result.source_hash != plan.source_hash() {
                return Err(conflict(
                    "validated result differs from frozen source and run",
                ));
            }
            return Ok(result.result_id.get());
        }
        RunState::Running => {
            return Err(PortError::Busy {
                message: "Translate run already has an active attempt".into(),
            });
        }
        RunState::Requested | RunState::Paused | RunState::Failed => {}
    }
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
            .map_err(|cause| auralis_translation::ProviderError(cause.to_string()))
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

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation run".into(),
        message: message.into(),
    }
}

fn storage(operation: &'static str, error: auralis_translation_sqlite::DbError) -> PortError {
    PortError::Storage {
        operation,
        message: error.to_string(),
    }
}
