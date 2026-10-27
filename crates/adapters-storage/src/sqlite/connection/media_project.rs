use super::{PortError, SqlitePool, validate_columns};
pub(super) async fn migrate_and_validate(
    pool: &SqlitePool,
    schema_marker: i64,
) -> Result<(), PortError> {
    if schema_marker < 22 {
        let mut tx = pool
            .begin()
            .await
            .map_err(crate::sqlite::media_project::migration_error)?;
        sqlx::raw_sql(include_str!("../media_project_schema.sql"))
            .execute(&mut *tx)
            .await
            .map_err(crate::sqlite::media_project::migration_error)?;
        sqlx::query("PRAGMA user_version = 22")
            .execute(&mut *tx)
            .await
            .map_err(crate::sqlite::media_project::migration_error)?;
        tx.commit()
            .await
            .map_err(crate::sqlite::media_project::migration_error)?;
    }
    validate(pool).await
}

pub(super) async fn validate(pool: &SqlitePool) -> Result<(), PortError> {
    validate_columns(
        pool,
        "media_project_receipts",
        &[
            "project_id",
            "manifest_json",
            "manifest_sha256",
            "state",
            "backup_json",
        ],
    )
    .await?;
    validate_columns(
        pool,
        "media_run_sources",
        &["run_id", "asset_id", "project_id"],
    )
    .await?;
    validate_columns(
        pool,
        "media_copy_operations",
        &[
            "id",
            "project_id",
            "source_asset_id",
            "source_sha256",
            "asset_json",
            "staging_key",
            "final_key",
            "state",
            "completed_bytes",
            "total_bytes",
            "error",
        ],
    )
    .await?;
    Ok(())
}
