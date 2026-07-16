use super::error::DatabaseTransitionError;
use super::inspector::LegacySchemaVersion;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionStage {
    Started,
    BackupFinalized,
    NewDatabaseReady,
    OldDatabaseQuarantined,
    NewDatabaseActivated,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionManifest {
    pub operation_id: Uuid,
    pub legacy_version: LegacySchemaVersion,
    pub stage: TransitionStage,
    pub database_name: String,
    pub backup_directory: PathBuf,
    pub new_database_name: String,
    pub quarantine_directory: PathBuf,
}

impl TransitionManifest {
    pub async fn load(path: &Path) -> Result<Self, DatabaseTransitionError> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))
    }

    pub async fn save(&self, path: &Path) -> Result<(), DatabaseTransitionError> {
        let parent = path.parent().unwrap();
        if !parent.exists() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;
        }

        let tmp_path = path.with_extension("tmp");
        let data = serde_json::to_string_pretty(self).unwrap();

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .await
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;

        file.write_all(data.as_bytes())
            .await
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;
        file.sync_data()
            .await
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;

        fs::rename(&tmp_path, path)
            .await
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;

        // Best effort sync parent directory
        #[cfg(target_family = "unix")]
        {
            if let Ok(parent_dir) = std::fs::File::open(parent) {
                let _ = parent_dir.sync_data();
            }
        }
        // For Windows, directory sync is not supported natively by std::fs::File in the same way, but it's best effort.

        Ok(())
    }
}
