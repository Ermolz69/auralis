use async_trait::async_trait;
use auralis_translation::{LanguageCode, ReviewState, RunId, RunState, TranslationId};
use auralis_translation_sqlite::{RunSpec, SqliteConfig, TranslateDb};
use ports::error::PortError;
use ports::translation_links::TranslationLink;
use ports::translation_run_control::{
    TranslationRunControlPort, TranslationRunPhase, TranslationRunSnapshot,
};
use ports::translation_start_guard::TranslationStartGuard;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct TranslateRunControl {
    database_path: PathBuf,
}

impl TranslateRunControl {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

#[async_trait]
impl TranslationRunControlPort for TranslateRunControl {
    async fn capture_start(
        &self,
        link: &TranslationLink,
    ) -> Result<TranslationStartGuard, PortError> {
        let database_path = self.database_path.clone();
        let link = link.clone();
        tokio::task::spawn_blocking(move || {
            let run_id = link
                .active_run_id
                .ok_or_else(|| conflict("translation has no active run"))?;
            let (database, run_id, _) = open_run(&database_path, &link, run_id)?;
            let guard = database
                .capture_attempt_start(run_id)
                .map_err(|error| database_error("capture_translation_start", error))?;
            let phase = match guard.state() {
                RunState::Requested => TranslationRunPhase::Requested,
                RunState::Paused => TranslationRunPhase::Paused,
                RunState::Failed => TranslationRunPhase::Failed,
                _ => return Err(conflict("run cannot admit a new attempt")),
            };
            Ok(TranslationStartGuard {
                run_id: run_id.get(),
                phase,
                control_revision: guard.control_revision(),
            })
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
    async fn snapshot(&self, link: &TranslationLink) -> Result<TranslationRunSnapshot, PortError> {
        let run_id = link
            .active_run_id
            .ok_or_else(|| conflict("translation has no active run"))?;
        self.snapshot_for_run(link, run_id).await
    }

    async fn snapshot_for_run(
        &self,
        link: &TranslationLink,
        run_id: Uuid,
    ) -> Result<TranslationRunSnapshot, PortError> {
        let database_path = self.database_path.clone();
        let link = link.clone();
        tokio::task::spawn_blocking(move || snapshot(&database_path, &link, run_id))
            .await
            .map_err(|error| PortError::Unexpected {
                message: error.to_string(),
            })?
    }

    async fn request_pause(&self, link: &TranslationLink) -> Result<(), PortError> {
        let database_path = self.database_path.clone();
        let link = link.clone();
        tokio::task::spawn_blocking(move || {
            let active_run_id = link
                .active_run_id
                .ok_or_else(|| conflict("translation has no active run"))?;
            let (database, run_id, _) = open_run(&database_path, &link, active_run_id)?;
            database
                .request_pause(run_id)
                .map_err(|error| database_error("request_translation_pause", error))
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
}

fn snapshot(
    database_path: &Path,
    link: &TranslationLink,
    requested_run_id: Uuid,
) -> Result<TranslationRunSnapshot, PortError> {
    let (database, run_id, run) = open_run(database_path, link, requested_run_id)?;
    let state = database
        .run_state(run_id)
        .map_err(|error| database_error("read_translation_run_state", error))?;
    let (result_id, needs_review) = if state == RunState::Validated {
        let result = database
            .result_for_run(run_id)
            .map_err(|error| database_error("read_translation_result", error))?;
        if result.run_id != run_id || result.source_hash.to_string() != link.source_sha256 {
            return Err(conflict("stored result does not match the project link"));
        }
        (
            Some(result.result_id.get()),
            Some(result.review_state == ReviewState::NeedsReview),
        )
    } else {
        (None, None)
    };
    let phase = match state {
        RunState::Requested => TranslationRunPhase::Requested,
        RunState::Running => TranslationRunPhase::Running,
        RunState::Paused => TranslationRunPhase::Paused,
        RunState::Failed => TranslationRunPhase::Failed,
        RunState::Validated => TranslationRunPhase::Validated,
    };
    Ok(TranslationRunSnapshot {
        run_id: run_id.get(),
        phase,
        pause_requested: database
            .pause_requested(run_id)
            .map_err(|error| database_error("read_translation_pause", error))?,
        saved_blocks: database
            .checkpoints(run_id)
            .map_err(|error| database_error("read_translation_checkpoints", error))?
            .len(),
        total_blocks: run.blocks.len(),
        result_id,
        needs_review,
    })
}

fn open_run(
    database_path: &Path,
    link: &TranslationLink,
    requested_run_id: Uuid,
) -> Result<(TranslateDb, RunId, RunSpec), PortError> {
    let translation_id =
        TranslationId::new(link.translation_id).ok_or_else(|| conflict("translation ID is nil"))?;
    let run_id = RunId::new(requested_run_id).ok_or_else(|| conflict("run ID is nil"))?;
    if !database_path.is_file() {
        return Err(PortError::NotFound {
            resource: "Translate database".into(),
        });
    }
    let database = TranslateDb::open(database_path, SqliteConfig::default())
        .map_err(|error| database_error("open_translate_database", error))?;
    let translation = database
        .translation(translation_id)
        .map_err(|error| database_error("read_translate_translation", error))?;
    if translation.project_id.as_deref() != Some(link.project_id.to_string().as_str())
        || translation.source_artifact_id.as_deref()
            != Some(link.source_artifact_id.to_string().as_str())
        || translation.source_hash.to_string() != link.source_sha256
        || translation.source_format != link.source_format
        || translation.language_pair.source()
            != match link.source_language.as_str() {
                "zh" => LanguageCode::Chinese,
                "ja" => LanguageCode::Japanese,
                _ => return Err(conflict("unsupported project link language")),
            }
        || translation.language_pair.target() != LanguageCode::Russian
        || link.target_language != "ru"
    {
        return Err(conflict(
            "stored translation does not match the project link",
        ));
    }
    let run = database
        .run(run_id)
        .map_err(|error| database_error("read_translate_run", error))?;
    if run.translation_id != translation_id || run.source_hash.to_string() != link.source_sha256 {
        return Err(conflict("stored run does not match the project link"));
    }
    Ok((database, run_id, run))
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation run".into(),
        message: message.into(),
    }
}

fn database_error(
    operation: &'static str,
    error: auralis_translation_sqlite::DbError,
) -> PortError {
    match error {
        auralis_translation_sqlite::DbError::Conflict(reason) => conflict(reason),
        other => PortError::Storage {
            operation,
            message: other.to_string(),
        },
    }
}
