use ports::error::PortError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::path::Path;

pub(crate) const SCHEMA: &str = concat!(
    include_str!("schema.sql"),
    include_str!("youtube_import_schema.sql"),
    include_str!("ui_preferences_schema.sql"),
    include_str!("translation_links_schema.sql"),
    include_str!("translation_run_intents_schema.sql"),
    include_str!("translation_host_jobs_schema.sql"),
    include_str!("translation_publication_revisions_schema.sql"),
    include_str!("translation_edit_intents_schema.sql"),
    include_str!("speech_run_schema.sql")
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
    if schema_marker >= 4 {
        expected_tables.push("youtube_imports");
    }
    if schema_marker >= 5 {
        expected_tables.extend(["project_pins", "ui_migrations", "ui_settings"]);
    }
    if schema_marker >= 6 {
        expected_tables.extend(["project_translations", "translation_publications"]);
    }
    if schema_marker >= 7 {
        expected_tables.push("translation_run_intents");
    }
    if schema_marker >= 8 {
        expected_tables.push("translation_host_jobs");
    }
    if schema_marker >= 10 {
        expected_tables.push("translation_edit_intents");
    }
    if schema_marker >= 11 {
        expected_tables.extend([
            "speech_run_intents",
            "speech_host_attempts",
            "speech_host_checkpoints",
        ]);
    }
    expected_tables.sort_unstable();
    if table_names != expected_tables || !(1..=11).contains(&schema_marker) {
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
    if schema_marker < 5 {
        super::ui_preferences_migration::migrate(pool).await?;
    }
    if schema_marker < 6 {
        super::translation_links_migration::migrate(pool).await?;
    }
    if schema_marker < 7 {
        super::translation_run_intents_migration::migrate(pool).await?;
    }
    if schema_marker < 8 {
        super::translation_host_jobs_migration::migrate(pool).await?;
    }
    if schema_marker < 9 {
        super::translation_publication_revisions_migration::migrate(pool).await?;
    }
    if schema_marker < 10 {
        super::translation_edit_intents_migration::migrate(pool).await?;
    }
    if schema_marker < 11 {
        super::speech_run_migration::migrate(pool).await?;
    }
    validate_columns(pool, "ui_settings", &["key", "value", "revision"]).await?;
    validate_columns(pool, "project_pins", &["project_id", "pinned", "revision"]).await?;
    validate_columns(pool, "ui_migrations", &["key"]).await?;
    validate_columns(
        pool,
        "youtube_imports",
        &["project_id", "request_key", "revision", "payload_json"],
    )
    .await?;
    validate_columns(
        pool,
        "project_translations",
        &[
            "translation_id",
            "project_id",
            "source_artifact_id",
            "source_sha256",
            "source_format",
            "source_language",
            "target_language",
            "active_run_id",
            "selected_result_id",
            "output_artifact_id",
            "revision",
            "created_at",
            "updated_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "translation_publications",
        &[
            "translation_id",
            "result_id",
            "run_id",
            "artifact_id",
            "source_sha256",
            "expected_link_revision",
            "state",
            "needs_review",
            "created_at",
            "updated_at",
            "result_revision",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "translation_run_intents",
        &[
            "run_id",
            "translation_id",
            "profile_manifest_json",
            "profile_sha256",
            "max_target_segments",
            "context_before_segments",
            "context_after_segments",
            "parser_version",
            "policy_fingerprint",
            "created_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "translation_host_jobs",
        &["job_id", "run_id", "created_at", "ended_at"],
    )
    .await?;

    validate_columns(
        pool,
        "translation_edit_intents",
        &[
            "result_id",
            "translation_id",
            "project_id",
            "base_result_id",
            "run_id",
            "expected_head_result_id",
            "expected_link_revision",
            "request_sha256",
            "created_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "speech_run_intents",
        &[
            "run_id",
            "project_id",
            "plan_json",
            "plan_sha256",
            "profile_id",
            "profile_sha256",
            "expected_utterances",
            "expected_chunks",
            "created_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "speech_host_attempts",
        &["attempt_id", "run_id", "job_id", "created_at", "ended_at"],
    )
    .await?;
    validate_columns(
        pool,
        "speech_host_checkpoints",
        &[
            "attempt_id",
            "sequence",
            "utterance_id",
            "cue_id",
            "chunk_id",
            "cache_key",
            "audio_sha256",
            "duration_ms",
            "origin",
            "saved_at",
        ],
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
