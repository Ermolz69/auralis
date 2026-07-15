use ports::error::PortError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;

pub async fn connect_sqlite<P: AsRef<Path>>(db_path: P) -> Result<SqlitePool, PortError> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to connect to sqlite db: {}", e),
        })?;

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
