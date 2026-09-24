use async_trait::async_trait;
use auralis_translation::{RunId, RunState};
use auralis_translation_sqlite::{DbError, SqliteConfig, TranslateDb};
use domain::job::JobId;
use ports::error::PortError;
use ports::recovered_translation_run::RecoveredTranslationRun;
use ports::translation_run_control::TranslationRunPhase;
use ports::translation_run_recovery::TranslationRunRecoveryPort;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct TranslateRunRecovery {
    database_path: PathBuf,
}

impl TranslateRunRecovery {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

#[async_trait]
impl TranslationRunRecoveryPort for TranslateRunRecovery {
    async fn recover_for_host(&self, run_id: Uuid, job_id: &JobId) -> Result<bool, PortError> {
        let database_path = self.database_path.clone();
        let job_id = job_id.to_string();
        tokio::task::spawn_blocking(move || recover(&database_path, run_id, &job_id))
            .await
            .map_err(|error| PortError::Unexpected {
                message: error.to_string(),
            })?
    }

    async fn state(&self, run_id: Uuid) -> Result<RecoveredTranslationRun, PortError> {
        let database_path = self.database_path.clone();
        tokio::task::spawn_blocking(move || read_state(&database_path, run_id))
            .await
            .map_err(|error| PortError::Unexpected {
                message: error.to_string(),
            })?
    }
}

fn recover(database_path: &Path, run_id: Uuid, job_id: &str) -> Result<bool, PortError> {
    let (mut database, run_id) = open(database_path, run_id)?;
    database
        .recover_interrupted_for_host(run_id, job_id)
        .map_err(map_db)
}

fn read_state(database_path: &Path, run_id: Uuid) -> Result<RecoveredTranslationRun, PortError> {
    let (database, run_id) = open(database_path, run_id)?;
    let phase = match database.run_state(run_id).map_err(map_db)? {
        RunState::Requested => TranslationRunPhase::Requested,
        RunState::Running => TranslationRunPhase::Running,
        RunState::Paused => TranslationRunPhase::Paused,
        RunState::Failed => TranslationRunPhase::Failed,
        RunState::Validated => TranslationRunPhase::Validated,
    };
    let total_blocks = database.run(run_id).map_err(map_db)?.blocks.len();
    let saved_blocks = database.checkpoints(run_id).map_err(map_db)?.len();
    if total_blocks == 0 || saved_blocks > total_blocks {
        return Err(PortError::Conflict {
            resource: "translation run".into(),
            message: "stored checkpoint count differs from block plan".into(),
        });
    }
    Ok(RecoveredTranslationRun {
        run_id: run_id.get(),
        phase,
        saved_blocks,
        total_blocks,
    })
}

fn open(database_path: &Path, run_id: Uuid) -> Result<(TranslateDb, RunId), PortError> {
    let run_id = RunId::new(run_id).ok_or_else(|| PortError::InvalidSource {
        message: "run ID is nil".into(),
    })?;
    if !database_path.is_file() {
        return Err(PortError::NotFound {
            resource: "Translate database".into(),
        });
    }
    let database = TranslateDb::open(database_path, SqliteConfig::default()).map_err(map_db)?;
    Ok((database, run_id))
}

fn map_db(error: DbError) -> PortError {
    match error {
        DbError::Conflict(message) => PortError::Conflict {
            resource: "translation run".into(),
            message: message.into(),
        },
        other => PortError::Storage {
            operation: "recover_owned_translation_run",
            message: other.to_string(),
        },
    }
}
