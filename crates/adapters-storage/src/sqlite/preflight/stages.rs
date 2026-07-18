use super::error::DatabaseTransitionError;
use super::manifest::TransitionManifest;
use sqlx::sqlite::SqliteConnectOptions;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct TransitionPaths {
    db_path: PathBuf,
    db_root: PathBuf,
}

impl TransitionPaths {
    pub fn new(db_path: PathBuf) -> Result<Self, DatabaseTransitionError> {
        let db_root = db_path
            .parent()
            .ok_or_else(|| {
                DatabaseTransitionError::CorruptTransitionState(
                    "database path must have a parent directory".to_string(),
                )
            })?
            .to_path_buf();
        if db_path.file_name().is_none() {
            return Err(DatabaseTransitionError::CorruptTransitionState(
                "database path must include a filename".to_string(),
            ));
        }
        Ok(Self { db_path, db_root })
    }

    pub fn db_root(&self) -> &Path {
        &self.db_root
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn current_db_file(&self, manifest: &TransitionManifest, suffix: &str) -> PathBuf {
        self.db_root
            .join(format!("{}{}", manifest.database_name, suffix))
    }

    pub fn new_db_path(&self, manifest: &TransitionManifest) -> PathBuf {
        self.db_root.join(&manifest.new_database_name)
    }
}

pub async fn backup_legacy_database(
    paths: &TransitionPaths,
    manifest: &TransitionManifest,
) -> Result<(), DatabaseTransitionError> {
    if manifest
        .backup_directory
        .join(&manifest.database_name)
        .exists()
    {
        verify_sqlite_readonly(
            &manifest.backup_directory.join(&manifest.database_name),
            DatabaseTransitionError::BackupValidationFailed,
        )
        .await?;
        return Ok(());
    }

    let tmp_backup_dir = manifest.backup_directory.with_extension("tmp");
    if tmp_backup_dir.exists() {
        fs::remove_dir_all(&tmp_backup_dir)
            .await
            .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;
    }
    fs::create_dir_all(&tmp_backup_dir)
        .await
        .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;

    for suffix in ["", "-wal", "-shm"] {
        let src = paths.current_db_file(manifest, suffix);
        if src.exists() {
            let dst = tmp_backup_dir.join(format!("{}{}", manifest.database_name, suffix));
            fs::copy(&src, &dst)
                .await
                .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;
        }
    }

    sync_directory(&tmp_backup_dir).await?;
    verify_sqlite_readonly(
        &tmp_backup_dir.join(&manifest.database_name),
        DatabaseTransitionError::BackupValidationFailed,
    )
    .await?;

    let backup_manifest_path = tmp_backup_dir.join("manifest.json");
    let manifest_content = serde_json::to_string_pretty(manifest)
        .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;
    fs::write(&backup_manifest_path, manifest_content)
        .await
        .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;
    sync_directory(&tmp_backup_dir).await?;

    fs::rename(&tmp_backup_dir, &manifest.backup_directory)
        .await
        .map_err(|e| DatabaseTransitionError::BackupFailed(e.to_string()))?;
    if let Some(parent) = manifest.backup_directory.parent() {
        sync_directory(parent).await?;
    }
    Ok(())
}

pub async fn prepare_new_database(
    paths: &TransitionPaths,
    manifest: &TransitionManifest,
) -> Result<(), DatabaseTransitionError> {
    let new_db_path = paths.new_db_path(manifest);
    if new_db_path.exists() {
        verify_new_database(&new_db_path).await?;
        return Ok(());
    }

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

    checkpoint_pool(&pool, DatabaseTransitionError::NewDatabaseValidationFailed).await?;
    verify_pool(&pool, DatabaseTransitionError::NewDatabaseValidationFailed).await?;
    pool.close().await;

    verify_new_database(&new_db_path).await?;
    sync_directory(paths.db_root()).await?;
    Ok(())
}

pub async fn quarantine_old_database(
    paths: &TransitionPaths,
    manifest: &TransitionManifest,
) -> Result<(), DatabaseTransitionError> {
    if manifest
        .quarantine_directory
        .join(&manifest.database_name)
        .exists()
        && !paths.current_db_file(manifest, "").exists()
    {
        return Ok(());
    }

    fs::create_dir_all(&manifest.quarantine_directory)
        .await
        .map_err(|e| DatabaseTransitionError::IncompleteTransitionWith(e.to_string()))?;

    for suffix in ["-wal", "-shm", ""] {
        let src = paths.current_db_file(manifest, suffix);
        if src.exists() {
            let dst = manifest
                .quarantine_directory
                .join(format!("{}{}", manifest.database_name, suffix));
            if dst.exists() {
                fs::remove_file(&src).await.map_err(|e| {
                    DatabaseTransitionError::IncompleteTransitionWith(e.to_string())
                })?;
            } else {
                fs::rename(&src, &dst).await.map_err(|e| {
                    DatabaseTransitionError::IncompleteTransitionWith(e.to_string())
                })?;
            }
        }
    }

    sync_directory(&manifest.quarantine_directory).await?;
    sync_directory(paths.db_root()).await?;
    Ok(())
}

pub async fn activate_new_database(
    paths: &TransitionPaths,
    manifest: &TransitionManifest,
) -> Result<(), DatabaseTransitionError> {
    let src = paths.new_db_path(manifest);
    let dst = paths.current_db_file(manifest, "");

    if dst.exists() {
        verify_new_database(&dst).await?;
        return Ok(());
    }
    if !src.exists() {
        return Err(DatabaseTransitionError::ResumeMismatch(
            "new database is missing before activation".to_string(),
        ));
    }

    fs::rename(&src, &dst)
        .await
        .map_err(|e| DatabaseTransitionError::IncompleteTransitionWith(e.to_string()))?;
    sync_directory(paths.db_root()).await?;
    verify_new_database(&dst).await?;
    Ok(())
}

pub async fn cleanup_transition(
    manifest: &TransitionManifest,
    manifest_path: &Path,
) -> Result<(), DatabaseTransitionError> {
    if manifest.quarantine_directory.exists() {
        fs::remove_dir_all(&manifest.quarantine_directory)
            .await
            .map_err(|e| DatabaseTransitionError::CleanupFailed(e.to_string()))?;
    }
    match fs::remove_file(manifest_path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(DatabaseTransitionError::CleanupFailed(e.to_string())),
    }
}

pub async fn writable_checkpoint(db_path: &Path) -> Result<(), DatabaseTransitionError> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(false);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;
    checkpoint_pool(&pool, |_| DatabaseTransitionError::CheckpointBusy).await?;
    pool.close().await;
    Ok(())
}

async fn verify_new_database(db_path: &Path) -> Result<(), DatabaseTransitionError> {
    verify_sqlite_readonly(
        db_path,
        DatabaseTransitionError::NewDatabaseValidationFailed,
    )
    .await
}

async fn verify_sqlite_readonly<F>(db_path: &Path, map: F) -> Result<(), DatabaseTransitionError>
where
    F: Fn(String) -> DatabaseTransitionError + Copy,
{
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(false)
        .read_only(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| map(e.to_string()))?;
    verify_pool(&pool, map).await?;
    pool.close().await;
    Ok(())
}

async fn verify_pool<F>(pool: &sqlx::SqlitePool, map: F) -> Result<(), DatabaseTransitionError>
where
    F: Fn(String) -> DatabaseTransitionError + Copy,
{
    let quick_check_res: String = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_one(pool)
        .await
        .map_err(|e| map(e.to_string()))?;
    if quick_check_res != "ok" {
        return Err(map(format!("quick_check returned: {}", quick_check_res)));
    }

    let fk_rows: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await
        .map_err(|e| map(e.to_string()))?;
    if !fk_rows.is_empty() {
        return Err(map("foreign_key_check failed".to_string()));
    }
    Ok(())
}

async fn checkpoint_pool<F>(pool: &sqlx::SqlitePool, map: F) -> Result<(), DatabaseTransitionError>
where
    F: Fn(String) -> DatabaseTransitionError,
{
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct CheckpointResult {
        busy: i32,
        log: i32,
        checkpointed: i32,
    }

    let result: CheckpointResult = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
        .fetch_one(pool)
        .await
        .map_err(|e| map(e.to_string()))?;
    if result.busy != 0 {
        return Err(map("checkpoint busy".to_string()));
    }
    Ok(())
}

#[allow(unused_variables)]
pub async fn sync_directory(path: &Path) -> Result<(), DatabaseTransitionError> {
    #[cfg(target_family = "unix")]
    {
        let dir = std::fs::File::open(path)
            .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;
        dir.sync_data()
            .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;
    }
    Ok(())
}
