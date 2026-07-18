use super::error::DatabaseTransitionError;
use super::inspector::{DatabaseClassification, LegacyDatabaseInspector, LegacySchemaVersion};
use super::lock::TransitionLock;
use super::manifest::{TransitionManifest, TransitionStage};
use super::stages::{
    TransitionPaths, activate_new_database, backup_legacy_database, cleanup_transition,
    prepare_new_database, quarantine_old_database, writable_checkpoint,
};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct TransitionStateMachine {
    db_path: PathBuf,
    backup_dir: PathBuf,
    paths: Option<TransitionPaths>,
    init_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionFailpoint {
    Started,
    BackupFinalized,
    NewDatabaseReady,
    OldDatabaseQuarantined,
    NewDatabaseActivated,
}

impl TransitionStateMachine {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        let db_path = db_path.as_ref().to_path_buf();
        match Self::try_new(&db_path) {
            Ok(machine) => machine,
            Err(err) => Self {
                db_path,
                backup_dir: PathBuf::new(),
                paths: None,
                init_error: Some(err.to_string()),
            },
        }
    }

    fn try_new(db_path: impl AsRef<Path>) -> Result<Self, DatabaseTransitionError> {
        let db_path = db_path.as_ref().to_path_buf();
        let paths = TransitionPaths::new(db_path.clone())?;
        let backup_dir = paths.db_root().join("backups");
        Ok(Self {
            db_path,
            backup_dir,
            paths: Some(paths),
            init_error: None,
        })
    }

    pub async fn run(&self) -> Result<(), DatabaseTransitionError> {
        self.run_inner(None).await
    }

    #[cfg(test)]
    pub async fn run_with_failpoint(
        &self,
        failpoint: TransitionFailpoint,
    ) -> Result<(), DatabaseTransitionError> {
        self.run_inner(Some(failpoint)).await
    }

    async fn run_inner(
        &self,
        #[cfg_attr(not(test), allow(unused_variables))] failpoint: Option<TransitionFailpoint>,
    ) -> Result<(), DatabaseTransitionError> {
        let manifest_path = self.manifest_path();
        let paths = self.paths()?;
        let manifest = if manifest_path.exists() {
            Some(
                TransitionManifest::load_validated(
                    &manifest_path,
                    paths.db_root(),
                    &self.backup_dir,
                )
                .await?,
            )
        } else {
            None
        };

        let classification = LegacyDatabaseInspector::inspect(&self.db_path).await?;
        let manifest = match (manifest, classification) {
            (Some(manifest), _) => manifest,
            (None, DatabaseClassification::Missing)
            | (None, DatabaseClassification::Empty)
            | (None, DatabaseClassification::ManagedSqlx) => return Ok(()),
            (None, DatabaseClassification::Unknown) => {
                return Err(DatabaseTransitionError::UnknownSchema);
            }
            (None, DatabaseClassification::Corrupt) => {
                return Err(DatabaseTransitionError::CorruptDatabase(
                    "SQLite reported the file is corrupt or not a database".to_string(),
                ));
            }
            (None, DatabaseClassification::KnownLegacy(version)) => {
                self.new_manifest(paths, version).await?
            }
        };

        let lock_path = self.db_path.with_extension("sqlite.lock");
        let lock =
            TransitionLock::try_acquire(lock_path, manifest_path.clone(), manifest.operation_id)
                .await?;
        lock.release().await?;

        self.execute_transition(paths, manifest, &manifest_path, failpoint)
            .await
    }

    async fn new_manifest(
        &self,
        paths: &TransitionPaths,
        version: LegacySchemaVersion,
    ) -> Result<TransitionManifest, DatabaseTransitionError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default() // allow-fallback
            .as_secs();
        let operation_id = Uuid::new_v4();
        let database_name = self
            .db_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                DatabaseTransitionError::CorruptTransitionState(
                    "database path must include a valid filename".to_string(),
                )
            })?
            .to_string();

        let manifest = TransitionManifest {
            operation_id,
            legacy_version: version,
            stage: TransitionStage::Started,
            database_name: database_name.clone(),
            backup_directory: self
                .backup_dir
                .join(format!("auralis-{}-{}", timestamp, operation_id)),
            new_database_name: format!("{}.new-{}", database_name, operation_id),
            quarantine_directory: self
                .backup_dir
                .join(format!("quarantine-{}-{}", timestamp, operation_id)),
        };
        manifest.validate(paths.db_root(), &self.backup_dir)?;
        Ok(manifest)
    }

    async fn execute_transition(
        &self,
        paths: &TransitionPaths,
        mut manifest: TransitionManifest,
        manifest_path: &Path,
        failpoint: Option<TransitionFailpoint>,
    ) -> Result<(), DatabaseTransitionError> {
        self.verify_resume_files(paths, &manifest)?;
        manifest.save(manifest_path).await?;
        self.fail_after(TransitionStage::Started, failpoint)?;

        if manifest.stage == TransitionStage::Started {
            writable_checkpoint(paths.db_path()).await?;
            backup_legacy_database(paths, &manifest).await?;
            manifest.stage = TransitionStage::BackupFinalized;
            manifest.save(manifest_path).await?;
            self.fail_after(TransitionStage::BackupFinalized, failpoint)?;
        }

        if manifest.stage == TransitionStage::BackupFinalized {
            prepare_new_database(paths, &manifest).await?;
            manifest.stage = TransitionStage::NewDatabaseReady;
            manifest.save(manifest_path).await?;
            self.fail_after(TransitionStage::NewDatabaseReady, failpoint)?;
        }

        if manifest.stage == TransitionStage::NewDatabaseReady {
            quarantine_old_database(paths, &manifest).await?;
            manifest.stage = TransitionStage::OldDatabaseQuarantined;
            manifest.save(manifest_path).await?;
            self.fail_after(TransitionStage::OldDatabaseQuarantined, failpoint)?;
        }

        if manifest.stage == TransitionStage::OldDatabaseQuarantined {
            activate_new_database(paths, &manifest).await?;
            manifest.stage = TransitionStage::NewDatabaseActivated;
            manifest.save(manifest_path).await?;
            self.fail_after(TransitionStage::NewDatabaseActivated, failpoint)?;
        }

        if manifest.stage == TransitionStage::NewDatabaseActivated {
            cleanup_transition(&manifest, manifest_path).await?;
        }

        Ok(())
    }

    fn verify_resume_files(
        &self,
        paths: &TransitionPaths,
        manifest: &TransitionManifest,
    ) -> Result<(), DatabaseTransitionError> {
        match manifest.stage {
            TransitionStage::Started | TransitionStage::BackupFinalized => {
                if !paths.db_path().exists() {
                    return Err(DatabaseTransitionError::ResumeMismatch(
                        "legacy database is missing before quarantine".to_string(),
                    ));
                }
            }
            TransitionStage::NewDatabaseReady => {
                if !paths.db_path().exists() {
                    return Err(DatabaseTransitionError::ResumeMismatch(
                        "legacy database is missing before quarantine".to_string(),
                    ));
                }
                if !paths.new_db_path(manifest).exists() {
                    return Err(DatabaseTransitionError::ResumeMismatch(
                        "new database is missing before quarantine".to_string(),
                    ));
                }
            }
            TransitionStage::OldDatabaseQuarantined => {
                if !paths.new_db_path(manifest).exists()
                    && !paths.current_db_file(manifest, "").exists()
                {
                    return Err(DatabaseTransitionError::ResumeMismatch(
                        "no database is available for activation".to_string(),
                    ));
                }
            }
            TransitionStage::NewDatabaseActivated => {
                if !paths.current_db_file(manifest, "").exists() {
                    return Err(DatabaseTransitionError::ResumeMismatch(
                        "activated database is missing".to_string(),
                    ));
                }
            }
            TransitionStage::Completed => {}
        }
        Ok(())
    }

    fn manifest_path(&self) -> PathBuf {
        self.backup_dir.join("transition_manifest.json")
    }

    fn paths(&self) -> Result<&TransitionPaths, DatabaseTransitionError> {
        self.paths.as_ref().ok_or_else(|| {
            DatabaseTransitionError::CorruptTransitionState(
                self.init_error
                    .clone()
                    .unwrap_or_else(|| "database path is invalid".to_string()),
            )
        })
    }

    fn fail_after(
        &self,
        #[cfg_attr(not(test), allow(unused_variables))] stage: TransitionStage,
        #[cfg_attr(not(test), allow(unused_variables))] failpoint: Option<TransitionFailpoint>,
    ) -> Result<(), DatabaseTransitionError> {
        #[cfg(test)]
        if matches!(
            (stage, failpoint),
            (TransitionStage::Started, Some(TransitionFailpoint::Started))
                | (
                    TransitionStage::BackupFinalized,
                    Some(TransitionFailpoint::BackupFinalized)
                )
                | (
                    TransitionStage::NewDatabaseReady,
                    Some(TransitionFailpoint::NewDatabaseReady)
                )
                | (
                    TransitionStage::OldDatabaseQuarantined,
                    Some(TransitionFailpoint::OldDatabaseQuarantined)
                )
                | (
                    TransitionStage::NewDatabaseActivated,
                    Some(TransitionFailpoint::NewDatabaseActivated)
                )
        ) {
            return Err(DatabaseTransitionError::TransitionRecoveryFailed(format!(
                "test failpoint after {:?}",
                stage
            )));
        }
        Ok(())
    }
}
