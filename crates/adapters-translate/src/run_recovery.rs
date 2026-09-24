use async_trait::async_trait;
use auralis_translation::{RunId, RunState};
use auralis_translation_sqlite::{DbError, SqliteConfig, TranslateDb};
use domain::job::JobId;
use ports::error::PortError;
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

    async fn phase(&self, run_id: Uuid) -> Result<TranslationRunPhase, PortError> {
        let database_path = self.database_path.clone();
        tokio::task::spawn_blocking(move || read_phase(&database_path, run_id))
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

fn read_phase(database_path: &Path, run_id: Uuid) -> Result<TranslationRunPhase, PortError> {
    let (database, run_id) = open(database_path, run_id)?;
    match database.run_state(run_id).map_err(map_db)? {
        RunState::Requested => Ok(TranslationRunPhase::Requested),
        RunState::Running => Ok(TranslationRunPhase::Running),
        RunState::Paused => Ok(TranslationRunPhase::Paused),
        RunState::Failed => Ok(TranslationRunPhase::Failed),
        RunState::Validated => Ok(TranslationRunPhase::Validated),
    }
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
