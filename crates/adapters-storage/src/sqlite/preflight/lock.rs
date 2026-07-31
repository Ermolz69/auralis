use super::error::DatabaseTransitionError;
use super::manifest::{TransitionManifest, TransitionStage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub const TRANSITION_LOCK_STALE_AFTER_SECS: u64 = 15 * 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct TransitionLockData {
    pub operation_id: Uuid,
    pub pid: u32,
    pub timestamp_sec: u64,
    pub manifest_path: PathBuf,
}

pub struct TransitionLock {
    lock_path: PathBuf,
}

impl TransitionLock {
    pub async fn try_acquire(
        lock_path: PathBuf,
        manifest_path: PathBuf,
        operation_id: Uuid,
    ) -> Result<Self, DatabaseTransitionError> {
        validate_lock_manifest_path(&manifest_path)?;
        let lock_data = TransitionLockData {
            operation_id,
            pid: std::process::id(),
            timestamp_sec: current_timestamp_sec()?,
            manifest_path: manifest_path.clone(),
        };
        let data_bytes = serde_json::to_vec(&lock_data)
            .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))?;

        match create_lock_file(&lock_path, &data_bytes).await {
            Ok(()) => Ok(Self { lock_path }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                reclaim_existing_lock(&lock_path).await?;
                match create_lock_file(&lock_path, &data_bytes).await {
                    Ok(()) => Ok(Self { lock_path }),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        Err(DatabaseTransitionError::LiveTransitionLock)
                    }
                    Err(e) => Err(DatabaseTransitionError::InspectionFailed(e.to_string())),
                }
            }
            Err(e) => Err(DatabaseTransitionError::InspectionFailed(e.to_string())),
        }
    }

    pub async fn release(self) -> Result<(), DatabaseTransitionError> {
        match fs::remove_file(&self.lock_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(DatabaseTransitionError::CleanupFailed(e.to_string())),
        }
    }
}

async fn create_lock_file(lock_path: &Path, data_bytes: &[u8]) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
        .await?;
    file.write_all(data_bytes).await?;
    file.sync_data().await
}

async fn reclaim_existing_lock(lock_path: &Path) -> Result<(), DatabaseTransitionError> {
    let existing_data = read_existing_lock(lock_path).await?;
    let age = current_timestamp_sec()?.saturating_sub(existing_data.timestamp_sec);
    let owner_is_current_process = existing_data.pid == std::process::id();
    if owner_is_current_process || age < TRANSITION_LOCK_STALE_AFTER_SECS {
        return Err(DatabaseTransitionError::LiveTransitionLock);
    }

    if existing_data.manifest_path.exists() {
        match TransitionManifest::load(&existing_data.manifest_path).await {
            Ok(manifest)
                if manifest.stage != TransitionStage::Completed
                    && age < TRANSITION_LOCK_STALE_AFTER_SECS * 2 =>
            {
                return Err(DatabaseTransitionError::LiveTransitionLock);
            }
            Ok(_) => {}
            Err(err) => return Err(err),
        }
    }

    fs::remove_file(lock_path)
        .await
        .map_err(|e| DatabaseTransitionError::StaleLockReclaimFailed(e.to_string()))
}

async fn read_existing_lock(path: &Path) -> Result<TransitionLockData, DatabaseTransitionError> {
    let mut existing_lock = fs::File::open(path)
        .await
        .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;
    let mut content = Vec::new();
    existing_lock
        .read_to_end(&mut content)
        .await
        .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;

    let data: TransitionLockData = serde_json::from_slice(&content)
        .map_err(|e| DatabaseTransitionError::CorruptTransitionLock(e.to_string()))?;
    validate_lock_payload(&data)?;
    Ok(data)
}

fn validate_lock_payload(data: &TransitionLockData) -> Result<(), DatabaseTransitionError> {
    if data.pid == 0 || data.timestamp_sec == 0 {
        return Err(DatabaseTransitionError::CorruptTransitionLock(
            "lock payload is missing required owner fields".to_string(),
        ));
    }
    validate_lock_manifest_path(&data.manifest_path)
}

fn validate_lock_manifest_path(path: &Path) -> Result<(), DatabaseTransitionError> {
    if !path.is_absolute() || path.file_name().and_then(|name| name.to_str()).is_none() {
        return Err(DatabaseTransitionError::CorruptTransitionLock(
            "lock payload manifest path must be an absolute file path".to_string(),
        ));
    }
    Ok(())
}

fn current_timestamp_sec() -> Result<u64, DatabaseTransitionError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| DatabaseTransitionError::TransitionRecoveryFailed(e.to_string()))
        .map(|duration| duration.as_secs())
}
