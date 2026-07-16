use super::error::DatabaseTransitionError;
use super::inspector::{DatabaseClassification, LegacyDatabaseInspector};
use super::lock::TransitionLock;
use super::manifest::{TransitionManifest, TransitionStage};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

pub struct TransitionStateMachine {
    db_path: PathBuf,
    backup_dir: PathBuf,
}

impl TransitionStateMachine {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        let db_path = db_path.as_ref().to_path_buf();
        let parent = db_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        let backup_dir = parent.join("backups");
        Self {
            db_path,
            backup_dir,
        }
    }

    pub async fn run(&self) -> Result<(), DatabaseTransitionError> {
        let classification = LegacyDatabaseInspector::inspect(&self.db_path).await?;

        match classification {
            DatabaseClassification::Missing
            | DatabaseClassification::Empty
            | DatabaseClassification::ManagedSqlx => {
                // Nothing to transition
                return Ok(());
            }
            DatabaseClassification::Unknown => {
                return Err(DatabaseTransitionError::UnknownSchema);
            }
            DatabaseClassification::Corrupt => {
                return Err(DatabaseTransitionError::CorruptDatabase(
                    "SQLite reported the file is corrupt or not a database".to_string(),
                ));
            }
            DatabaseClassification::KnownLegacy(version) => {
                let lock_path = self.db_path.with_extension("sqlite.lock");
                let manifest_path = self.backup_dir.join("transition_manifest.json");

                // If a lock is held, we check if we can resume.
                let mut lock =
                    TransitionLock::try_acquire(lock_path, manifest_path.clone(), Uuid::new_v4())
                        .await?;

                // Re-inspect under lock
                let re_classification = LegacyDatabaseInspector::inspect(&self.db_path).await?;
                if re_classification != DatabaseClassification::KnownLegacy(version) {
                    lock.release().await;
                    return Ok(());
                }

                // Create manifest
                let manifest = if manifest_path.exists() {
                    TransitionManifest::load(&manifest_path).await?
                } else {
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let op_id = Uuid::new_v4();

                    TransitionManifest {
                        operation_id: op_id,
                        legacy_version: version,
                        stage: TransitionStage::Started,
                        database_name: self
                            .db_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        backup_directory: self
                            .backup_dir
                            .join(format!("auralis-{}-{}", timestamp, op_id)),
                        new_database_name: format!(
                            "{}.new-{}",
                            self.db_path.file_name().unwrap().to_string_lossy(),
                            op_id
                        ),
                        quarantine_directory: self
                            .backup_dir
                            .join(format!("quarantine-{}-{}", timestamp, op_id)),
                    }
                };

                let result = self
                    .execute_transition(manifest.clone(), &manifest_path)
                    .await;

                // Cleanup lock
                lock.release().await;

                result?;
            }
        }
        Ok(())
    }

    async fn execute_transition(
        &self,
        mut manifest: TransitionManifest,
        manifest_path: &Path,
    ) -> Result<(), DatabaseTransitionError> {
        manifest.save(manifest_path).await?;

        if manifest.stage == TransitionStage::Started {
            self.writable_checkpoint().await?;
            self.perform_backup(&manifest).await?;
            manifest.stage = TransitionStage::BackupFinalized;
            manifest.save(manifest_path).await?;
        }

        if manifest.stage == TransitionStage::BackupFinalized {
            self.prepare_new_database(&manifest).await?;
            manifest.stage = TransitionStage::NewDatabaseReady;
            manifest.save(manifest_path).await?;
        }

        if manifest.stage == TransitionStage::NewDatabaseReady {
            self.quarantine_old_database(&manifest).await?;
            manifest.stage = TransitionStage::OldDatabaseQuarantined;
            manifest.save(manifest_path).await?;
        }

        if manifest.stage == TransitionStage::OldDatabaseQuarantined {
            self.activate_new_database(&manifest).await?;
            manifest.stage = TransitionStage::NewDatabaseActivated;
            manifest.save(manifest_path).await?;
        }

        if manifest.stage == TransitionStage::NewDatabaseActivated {
            self.cleanup(&manifest, manifest_path).await?;
        }

        Ok(())
    }

    async fn writable_checkpoint(&self) -> Result<(), DatabaseTransitionError> {
        let options = SqliteConnectOptions::new()
            .filename(&self.db_path)
            .create_if_missing(false); // Do not create, we just want to checkpoint it.

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;

        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct CheckpointResult {
            busy: i32,
            log: i32,
            checkpointed: i32,
        }

        let result: CheckpointResult = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;

        pool.close().await;

        if result.busy != 0 {
            return Err(DatabaseTransitionError::CheckpointBusy);
        }

        Ok(())
    }

    async fn perform_backup(
        &self,
        manifest: &TransitionManifest,
    ) -> Result<(), DatabaseTransitionError> {
        let tmp_backup_dir = manifest.backup_directory.with_extension("tmp");
        if tmp_backup_dir.exists() {
            fs::remove_dir_all(&tmp_backup_dir).await.ok();
        }
        fs::create_dir_all(&tmp_backup_dir)
            .await
            .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;

        // Copy files
        for suffix in ["", "-wal", "-shm"] {
            let src = self
                .db_path
                .with_file_name(format!("{}{}", manifest.database_name, suffix));
            if src.exists() {
                let dst = tmp_backup_dir.join(format!("{}{}", manifest.database_name, suffix));
                fs::copy(&src, &dst)
                    .await
                    .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;
            }
        }

        self.sync_directory(&tmp_backup_dir).await;

        // Quick check
        let backup_db_path = tmp_backup_dir.join(&manifest.database_name);
        let options = SqliteConnectOptions::new()
            .filename(&backup_db_path)
            .create_if_missing(false)
            .read_only(true);

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| DatabaseTransitionError::BackupValidationFailed(e.to_string()))?;

        let quick_check_res: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseTransitionError::BackupValidationFailed(e.to_string()))?;

        pool.close().await;

        if quick_check_res != "ok" {
            return Err(DatabaseTransitionError::BackupValidationFailed(format!(
                "quick_check returned: {}",
                quick_check_res
            )));
        }

        // Write manifest
        let backup_manifest_path = tmp_backup_dir.join("manifest.json");
        let manifest_content = serde_json::to_string_pretty(manifest).unwrap();
        fs::write(&backup_manifest_path, manifest_content)
            .await
            .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;

        self.sync_directory(&tmp_backup_dir).await;

        // Rename
        fs::rename(&tmp_backup_dir, &manifest.backup_directory)
            .await
            .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;
        self.sync_directory(&self.backup_dir).await;

        Ok(())
    }

    async fn prepare_new_database(
        &self,
        manifest: &TransitionManifest,
    ) -> Result<(), DatabaseTransitionError> {
        let parent = self.db_path.parent().unwrap();
        let new_db_path = parent.join(&manifest.new_database_name);

        let options = SqliteConnectOptions::new()
            .filename(&new_db_path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| DatabaseTransitionError::FreshDatabaseCreationFailed(e.to_string()))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| DatabaseTransitionError::FreshDatabaseCreationFailed(e.to_string()))?;

        // Checkpoint the new DB to flush WAL
        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct CheckpointResult {
            busy: i32,
            log: i32,
            checkpointed: i32,
        }

        let cp_res: CheckpointResult = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseTransitionError::NewDatabaseValidationFailed(e.to_string()))?;

        if cp_res.busy != 0 {
            return Err(DatabaseTransitionError::NewDatabaseValidationFailed(
                "Checkpoint busy on new db".to_string(),
            ));
        }

        // Validation
        let quick_check_res: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&pool)
            .await
            .map_err(|e| DatabaseTransitionError::NewDatabaseValidationFailed(e.to_string()))?;

        if quick_check_res != "ok" {
            return Err(DatabaseTransitionError::NewDatabaseValidationFailed(
                format!("quick_check returned: {}", quick_check_res),
            ));
        }

        let fk_check: Vec<String> = sqlx::query_scalar("PRAGMA foreign_key_check")
            .fetch_all(&pool)
            .await
            .map_err(|e| DatabaseTransitionError::NewDatabaseValidationFailed(e.to_string()))?;

        if !fk_check.is_empty() {
            return Err(DatabaseTransitionError::NewDatabaseValidationFailed(
                "foreign_key_check failed".to_string(),
            ));
        }

        pool.close().await;

        // Verify read-only to ensure WAL isn't strictly required to read
        let ro_options = SqliteConnectOptions::new()
            .filename(&new_db_path)
            .create_if_missing(false)
            .read_only(true);
        let ro_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(ro_options)
            .await
            .map_err(|e| DatabaseTransitionError::NewDatabaseValidationFailed(e.to_string()))?;
        ro_pool.close().await;

        Ok(())
    }

    async fn quarantine_old_database(
        &self,
        manifest: &TransitionManifest,
    ) -> Result<(), DatabaseTransitionError> {
        let parent = self.db_path.parent().unwrap();
        if !manifest.quarantine_directory.exists() {
            fs::create_dir_all(&manifest.quarantine_directory)
                .await
                .map_err(|_e| DatabaseTransitionError::IncompleteTransition)?;
        }

        // Move wal and shm first
        for suffix in ["-wal", "-shm"] {
            let src = parent.join(format!("{}{}", manifest.database_name, suffix));
            if src.exists() {
                let dst = manifest
                    .quarantine_directory
                    .join(format!("{}{}", manifest.database_name, suffix));
                fs::rename(&src, &dst)
                    .await
                    .map_err(|_| DatabaseTransitionError::IncompleteTransition)?;
            }
        }

        // Move main file last
        let src = parent.join(&manifest.database_name);
        if src.exists() {
            let dst = manifest.quarantine_directory.join(&manifest.database_name);
            fs::rename(&src, &dst)
                .await
                .map_err(|_| DatabaseTransitionError::IncompleteTransition)?;
        }

        self.sync_directory(&manifest.quarantine_directory).await;
        self.sync_directory(parent).await;

        Ok(())
    }

    async fn activate_new_database(
        &self,
        manifest: &TransitionManifest,
    ) -> Result<(), DatabaseTransitionError> {
        let parent = self.db_path.parent().unwrap();
        let src = parent.join(&manifest.new_database_name);
        let dst = parent.join(&manifest.database_name);

        fs::rename(&src, &dst)
            .await
            .map_err(|_| DatabaseTransitionError::IncompleteTransition)?;

        self.sync_directory(parent).await;

        Ok(())
    }

    async fn cleanup(
        &self,
        manifest: &TransitionManifest,
        manifest_path: &Path,
    ) -> Result<(), DatabaseTransitionError> {
        if manifest.quarantine_directory.exists() {
            let _ = fs::remove_dir_all(&manifest.quarantine_directory).await;
        }
        let _ = fs::remove_file(manifest_path).await;
        Ok(())
    }

    async fn sync_directory(&self, _path: &Path) {
        #[cfg(target_family = "unix")]
        {
            if let Ok(dir) = std::fs::File::open(path) {
                let _ = dir.sync_data();
            }
        }
        // Best effort for other platforms
    }
}
