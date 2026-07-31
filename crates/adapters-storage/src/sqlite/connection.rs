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
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to connect to sqlite db", e))
}

pub async fn connect_sqlite<P: AsRef<Path>>(db_path: P) -> Result<SqlitePool, PortError> {
    let db_path = db_path.as_ref();

    // Run robust preflight
    let preflight = crate::sqlite::preflight::state_machine::TransitionStateMachine::new(db_path);
    preflight.run().await?;

    let pool = create_pool(db_path).await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| PortError::Storage {
            operation: "run_sqlite_migrations",
            message: e.to_string(),
        })?;

    // Note: Runtime backfills must be run manually after this function,
    // passing the appropriate workspace_root to them.

    Ok(pool)
}
