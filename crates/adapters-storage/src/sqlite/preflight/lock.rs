use super::error::DatabaseTransitionError;
use super::manifest::{TransitionManifest, TransitionStage};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

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
        let lock_data = TransitionLockData {
            operation_id,
            pid: std::process::id(),
            timestamp_sec: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            manifest_path: manifest_path.clone(),
        };
        let data_bytes = serde_json::to_vec(&lock_data).unwrap();

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .await
        {
            Ok(mut file) => {
                file.write_all(&data_bytes)
                    .await
                    .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;
                file.sync_data()
                    .await
                    .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;
                Ok(Self { lock_path })
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Lock exists. Read it.
                let mut existing_lock = fs::File::open(&lock_path)
                    .await
                    .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;
                let mut content = Vec::new();
                existing_lock
                    .read_to_end(&mut content)
                    .await
                    .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;

                let existing_data: Result<TransitionLockData, _> = serde_json::from_slice(&content);

                match existing_data {
                    Ok(data) => {
                        // Check if manifest exists and what stage it is in.
                        // If the manifest doesn't exist, the previous process died immediately after creating the lock.
                        if !data.manifest_path.exists() {
                            tracing::warn!(
                                "Found stale transition lock with no manifest. Removing and retrying."
                            );
                            fs::remove_file(&lock_path).await.ok();
                            return Box::pin(Self::try_acquire(
                                lock_path,
                                manifest_path,
                                operation_id,
                            ))
                            .await;
                        }

                        // If the manifest exists, we check its stage. If it's Completed, it's a stale lock from a successful transition.
                        if let Ok(manifest) = TransitionManifest::load(&data.manifest_path).await
                            && manifest.stage == TransitionStage::Completed
                        {
                            tracing::warn!(
                                "Found stale transition lock for a completed transition. Removing and retrying."
                            );
                            fs::remove_file(&lock_path).await.ok();
                            fs::remove_file(&data.manifest_path).await.ok();
                            return Box::pin(Self::try_acquire(
                                lock_path,
                                manifest_path,
                                operation_id,
                            ))
                            .await;
                        }

                        // Otherwise, it's a valid lock or an interrupted transition that we should recover.
                        Err(DatabaseTransitionError::TransitionLocked)
                    }
                    Err(_) => {
                        // Corrupted lock file.
                        tracing::warn!("Found corrupted transition lock. Removing and retrying.");
                        fs::remove_file(&lock_path).await.ok();
                        Box::pin(Self::try_acquire(lock_path, manifest_path, operation_id)).await
                    }
                }
            }
            Err(e) => Err(DatabaseTransitionError::InspectionFailed(e.to_string())),
        }
    }

    pub async fn release(self) {
        let _ = fs::remove_file(&self.lock_path).await;
    }
}
