use super::error::DatabaseTransitionError;
use super::inspector::LegacySchemaVersion;
use serde::{Deserialize, Serialize};
use std::path::Component;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
            .map_err(|e| DatabaseTransitionError::CorruptTransitionState(e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| DatabaseTransitionError::CorruptTransitionState(e.to_string()))
    }

    pub async fn load_validated(
        path: &Path,
        database_root: &Path,
        backup_root: &Path,
    ) -> Result<Self, DatabaseTransitionError> {
        let manifest = Self::load(path).await?;
        manifest.validate(database_root, backup_root)?;
        Ok(manifest)
    }

    pub async fn save(&self, path: &Path) -> Result<(), DatabaseTransitionError> {
        let parent = path.parent().ok_or_else(|| {
            DatabaseTransitionError::TransitionRecoveryFailed(
                "manifest path has no parent directory".to_string(),
            )
        })?;
        if !parent.exists() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;
        }

        let tmp_path = path.with_extension("tmp");
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;

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

    pub fn validate(
        &self,
        database_root: &Path,
        backup_root: &Path,
    ) -> Result<(), DatabaseTransitionError> {
        validate_filename("database_name", &self.database_name)?;
        validate_filename("new_database_name", &self.new_database_name)?;
        validate_contained_path("backup_directory", &self.backup_directory, backup_root)?;
        validate_contained_path(
            "quarantine_directory",
            &self.quarantine_directory,
            backup_root,
        )?;

        let new_database_path = database_root.join(&self.new_database_name);
        validate_contained_path("new_database_path", &new_database_path, database_root)?;
        Ok(())
    }
}

fn validate_filename(label: &str, value: &str) -> Result<(), DatabaseTransitionError> {
    if value.is_empty()
        || Path::new(value).file_name().and_then(|name| name.to_str()) != Some(value)
        || Path::new(value).components().count() != 1
    {
        return Err(DatabaseTransitionError::CorruptTransitionState(format!(
            "{} must be a plain filename",
            label
        )));
    }
    Ok(())
}

fn validate_contained_path(
    label: &str,
    path: &Path,
    root: &Path,
) -> Result<(), DatabaseTransitionError> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(DatabaseTransitionError::CorruptTransitionState(format!(
            "{} escapes the transition root",
            label
        )));
    }

    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    if !candidate.starts_with(root) {
        return Err(DatabaseTransitionError::CorruptTransitionState(format!(
            "{} escapes the transition root",
            label
        )));
    }
    Ok(())
}
