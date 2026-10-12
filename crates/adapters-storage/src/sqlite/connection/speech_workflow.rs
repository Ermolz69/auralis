use super::{PortError, SqlitePool, validate_columns};

pub(super) async fn migrate_and_validate(
    pool: &SqlitePool,
    schema_marker: i64,
) -> Result<(), PortError> {
    if schema_marker < 15 {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_speech_workflow", e))?;
        sqlx::raw_sql(include_str!("../speech_workflow_schema.sql"))
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_speech_workflow", e))?;
        tx.commit()
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_speech_workflow", e))?;
    }
    if schema_marker < 16 {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_speech_discard", e))?;
        sqlx::raw_sql(include_str!("../speech_discard_schema.sql"))
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_speech_discard", e))?;
        tx.commit()
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_speech_discard", e))?;
    }
    validate_columns(
        pool,
        "speech_discarded_artifacts",
        &[
            "artifact_id",
            "run_id",
            "attempt_id",
            "final_key",
            "created_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "speech_text_reviews",
        &["decision_id", "project_id", "snapshot_json", "created_at"],
    )
    .await?;
    validate_columns(
        pool,
        "speech_publications",
        &[
            "run_id",
            "attempt_id",
            "publication_json",
            "ready",
            "created_at",
        ],
    )
    .await?;
    validate_columns(pool, "speech_run_reviews", &["run_id", "decision_id"]).await?;
    Ok(())
}
