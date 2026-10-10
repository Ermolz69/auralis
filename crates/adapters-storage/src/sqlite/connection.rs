use ports::error::PortError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::path::Path;

mod validation;
use validation::validate_existing_database;

pub(crate) const SCHEMA: &str = concat!(
    include_str!("schema.sql"),
    include_str!("youtube_import_schema.sql"),
    include_str!("ui_preferences_schema.sql"),
    include_str!("translation_links_schema.sql"),
    include_str!("translation_run_intents_schema.sql"),
    include_str!("translation_host_jobs_schema.sql"),
    include_str!("translation_publication_revisions_schema.sql"),
    include_str!("translation_edit_intents_schema.sql"),
    include_str!("speech_run_schema.sql"),
    include_str!("job_executions_schema.sql"),
    include_str!("translation_reviews_schema.sql")
);

pub(crate) async fn create_pool(db_path: &Path) -> Result<SqlitePool, PortError> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Full)
        .busy_timeout(std::time::Duration::from_secs(5))
        .journal_mode(SqliteJournalMode::Wal);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to connect to sqlite db", e))
}

pub async fn connect_sqlite<P: AsRef<Path>>(db_path: P) -> Result<SqlitePool, PortError> {
    let db_path = db_path.as_ref();
    let pool = create_pool(db_path).await?;

    if validate_existing_database(&pool).await? {
        let mut transaction = pool.begin().await.map_err(|e| PortError::Storage {
            operation: "initialize_sqlite_schema",
            message: e.to_string(),
        })?;
        sqlx::raw_sql(SCHEMA)
            .execute(&mut *transaction)
            .await
            .map_err(|e| PortError::Storage {
                operation: "initialize_sqlite_schema",
                message: e.to_string(),
            })?;
        transaction.commit().await.map_err(|e| PortError::Storage {
            operation: "initialize_sqlite_schema",
            message: e.to_string(),
        })?;
        validate_existing_database(&pool).await?;
    }

    Ok(pool)
}
