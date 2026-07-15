use ports::error::PortError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;

pub(crate) async fn create_pool(db_path: &Path) -> Result<SqlitePool, PortError> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to connect to sqlite db: {}", e),
        })
}

pub async fn connect_sqlite<P: AsRef<Path>>(db_path: P) -> Result<SqlitePool, PortError> {
    let db_path = db_path.as_ref();

    if db_path.exists() {
        // Preflight check using a temporary pool
        let pool = create_pool(db_path).await?;

        let has_migrations: bool = sqlx::query(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or_default()
        .is_some();

        let has_projects: bool =
            sqlx::query("SELECT 1 FROM sqlite_master WHERE type='table' AND name='projects'")
                .fetch_optional(&pool)
                .await
                .unwrap_or_default()
                .is_some();

        let has_user_tables: bool = sqlx::query(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or_default()
        .is_some();

        if !has_migrations {
            if has_projects {
                // Case 1: Handcrafted dev DB
                pool.close().await;

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let db_name = db_path.file_name().unwrap_or_default().to_string_lossy();
                let backup_dir = db_path.parent().unwrap_or(Path::new(""));
                let backup_path = backup_dir.join(format!("{}.backup_{}", db_name, timestamp));

                tracing::warn!(
                    "Legacy unmigrated dev database detected. Backing up to {} and recreating a fresh database...",
                    backup_path.display()
                );

                for suffix in ["", "-wal", "-shm"] {
                    let src = backup_dir.join(format!("{}{}", db_name, suffix));
                    let dst =
                        backup_dir.join(format!("{}{}.backup_{}", db_name, suffix, timestamp));

                    if src.exists() {
                        let mut retries = 5;
                        let mut last_err = None;
                        while retries > 0 {
                            match tokio::fs::rename(&src, &dst).await {
                                Ok(_) => {
                                    last_err = None;
                                    break;
                                }
                                Err(e) => {
                                    last_err = Some(e);
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                    retries -= 1;
                                }
                            }
                        }

                        if let Some(e) = last_err {
                            tracing::error!("Failed to rename {}: {}", src.display(), e);
                            if suffix.is_empty() {
                                return Err(PortError::Unexpected {
                                    message: format!("Failed to backup database file: {}", e),
                                });
                            }
                        }
                    }
                }
            } else if has_user_tables {
                // Case 2: Unknown schema
                pool.close().await;
                return Err(PortError::Unexpected {
                    message: "Unknown database schema detected (contains user tables but no _sqlx_migrations or projects). Please remove or backup the database manually.".to_string(),
                });
            }
        }
    }

    let pool = create_pool(db_path).await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to run sqlite migrations: {}", e),
        })?;

    // Note: Runtime backfills must be run manually after this function,
    // passing the appropriate workspace_root to them.

    Ok(pool)
}
