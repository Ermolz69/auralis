use ports::error::PortError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;

pub(crate) const SCHEMA: &str = concat!(
    include_str!("schema.sql"),
    include_str!("youtube_import_schema.sql")
);

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

/// Returns `true` only when a new database still needs its one-time initialization.
async fn validate_existing_database(pool: &SqlitePool) -> Result<bool, PortError> {
    let table_names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to inspect sqlite schema", e))?;

    if table_names.is_empty() {
        return Ok(true);
    }

    const FINAL_TABLES: [&str; 4] = ["artifacts", "jobs", "outbox_messages", "projects"];
    let schema_marker: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
        .map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to inspect sqlite schema marker", e)
        })?;
    let mut expected_tables = FINAL_TABLES.to_vec();
    if schema_marker == 4 {
        expected_tables.push("youtube_imports");
    }
    if table_names != expected_tables || ![1, 2, 3, 4].contains(&schema_marker) {
        return Err(non_final_schema_error("database"));
    }

    const FINAL_PROJECT_COLUMNS: [&str; 12] = [
        "id",
        "title",
        "status",
        "source_json",
        "metadata_json",
        "source_language",
        "target_language",
        "transcript_json",
        "active_job_id",
        "last_terminal_job_id",
        "created_at",
        "updated_at",
    ];
    let mut project_columns = FINAL_PROJECT_COLUMNS.to_vec();
    if schema_marker >= 2 {
        project_columns.push("revision");
    }
    if schema_marker >= 3 {
        project_columns.push("avatar_data_url");
    }
    validate_columns(pool, "projects", &project_columns).await?;

    const FINAL_JOB_COLUMNS: [&str; 13] = [
        "id",
        "revision",
        "project_id",
        "title",
        "kind",
        "status",
        "stage",
        "progress_json",
        "error_json",
        "created_at",
        "updated_at",
        "started_at",
        "finished_at",
    ];
    validate_columns(pool, "jobs", &FINAL_JOB_COLUMNS).await?;

    const FINAL_ARTIFACT_COLUMNS: [&str; 10] = [
        "id",
        "project_id",
        "kind",
        "location_kind",
        "location_value",
        "size_bytes",
        "state",
        "created_at",
        "updated_at",
        "ready_at",
    ];
    validate_columns(pool, "artifacts", &FINAL_ARTIFACT_COLUMNS).await?;

    const FINAL_OUTBOX_COLUMNS: [&str; 14] = [
        "id",
        "kind",
        "payload_json",
        "status",
        "attempts",
        "next_attempt_at",
        "locked_at",
        "locked_by",
        "last_error",
        "deduplication_key",
        "created_at",
        "updated_at",
        "aggregate_type",
        "aggregate_id",
    ];
    validate_columns(pool, "outbox_messages", &FINAL_OUTBOX_COLUMNS).await?;

    let artifact_sql: String = sqlx::query_scalar(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'artifacts'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to inspect artifacts schema", e)
    })?;
    if !artifact_sql.contains("location_kind = 'StorageKey'") {
        return Err(non_final_schema_error("artifacts table"));
    }

    if schema_marker < 3 {
        super::project_revision_migration::migrate(pool).await?;
    }
    if schema_marker < 4 {
        super::youtube_import_migration::migrate(pool).await?;
    }
    validate_columns(
        pool,
        "youtube_imports",
        &["project_id", "request_key", "revision", "payload_json"],
    )
    .await?;

    Ok(false)
}

async fn validate_columns(
    pool: &SqlitePool,
    table: &'static str,
    expected: &[&str],
) -> Result<(), PortError> {
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
            .bind(table)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                crate::sqlite::helpers::map_sqlite_error("Failed to inspect sqlite table", e)
            })?;

    if columns
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
    {
        Ok(())
    } else {
        Err(non_final_schema_error(table))
    }
}

fn non_final_schema_error(component: &str) -> PortError {
    PortError::Storage {
        operation: "validate_sqlite_schema",
        message: format!("The {component} does not use the current Auralis schema"),
    }
}
