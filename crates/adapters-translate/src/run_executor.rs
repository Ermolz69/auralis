mod attempt;
mod frozen_input;

use self::attempt::execute_attempt;
use self::frozen_input::prepare;
use async_trait::async_trait;
use auralis_translation::RunState;
use ports::committed_translation_progress::CommittedTranslationProgress;
use ports::error::PortError;
use ports::host_translation_execution::HostTranslationExecution;
use ports::translation_execution::TranslationExecutionPort;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
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
    let prepared = prepare(
        database_path,
        source_path,
        link,
        intent,
        options.allow_unchecked_profile,
    )?;
    match prepared.state {
        RunState::Validated => {
            let result = prepared
                .database
                .result_for_run(prepared.run_id)
                .map_err(|error| storage("read_translate_result", error))?;
            if result.run_id != prepared.run_id || result.source_hash != prepared.plan.source_hash()
            {
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
    execute_attempt(database_path, prepared, server_url, options)
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
