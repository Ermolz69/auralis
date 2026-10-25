use super::{PortError, SqlitePool, validate_columns};

pub(super) fn extend_expected_tables(tables: &mut Vec<&'static str>, marker: i64) {
    if marker >= 19 {
        tables.extend([
            "audio_assembly_intents",
            "audio_assembly_attempts",
            "audio_assembly_publications",
            "audio_assembly_discarded_artifacts",
            "audio_assembly_artifacts",
        ]);
    }
}

pub(super) async fn migrate_and_validate(pool: &SqlitePool, marker: i64) -> Result<(), PortError> {
    if marker < 19 {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
            crate::sqlite::helpers::map_sqlite_error("migrate_audio_assembly", error)
        })?;
        sqlx::raw_sql(include_str!("../audio_assembly_schema.sql"))
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                crate::sqlite::helpers::map_sqlite_error("migrate_audio_assembly", error)
            })?;
        transaction.commit().await.map_err(|error| {
            crate::sqlite::helpers::map_sqlite_error("migrate_audio_assembly", error)
        })?;
    }
    if marker < 20 {
        let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
            crate::sqlite::helpers::map_sqlite_error("migrate_video_assembly", error)
        })?;
        sqlx::raw_sql(include_str!("../video_assembly_migration.sql"))
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                crate::sqlite::helpers::map_sqlite_error("migrate_video_assembly", error)
            })?;
        transaction.commit().await.map_err(|error| {
            crate::sqlite::helpers::map_sqlite_error("migrate_video_assembly", error)
        })?;
    }
    validate_columns(
        pool,
        "audio_assembly_intents",
        &[
            "assembly_id",
            "project_id",
            "source_run_id",
            "source_attempt_id",
            "plan_json",
            "plan_sha256",
            "created_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "audio_assembly_attempts",
        &[
            "attempt_id",
            "assembly_id",
            "job_id",
            "created_at",
            "ended_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "audio_assembly_publications",
        &[
            "assembly_id",
            "attempt_id",
            "publication_json",
            "ready",
            "created_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "audio_assembly_discarded_artifacts",
        &[
            "artifact_id",
            "assembly_id",
            "attempt_id",
            "final_key",
            "created_at",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "audio_assembly_artifacts",
        &[
            "artifact_id",
            "assembly_id",
            "attempt_id",
            "role",
            "sha256",
            "size_bytes",
        ],
    )
    .await
}
