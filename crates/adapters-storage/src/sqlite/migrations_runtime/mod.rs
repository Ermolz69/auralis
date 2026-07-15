pub mod backfill_artifacts;
pub mod outbox_temp_paths;

use ports::error::PortError;
use sqlx::SqlitePool;

pub async fn run_runtime_backfills(
    pool: &SqlitePool,
    workspace_root: &std::path::Path,
) -> Result<(), PortError> {
    backfill_artifacts::run(pool).await?;

    let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
        message: format!("Failed to begin tx for runtime migration: {}", e),
    })?;

    outbox_temp_paths::migrate_delete_temp_paths(&mut tx, workspace_root).await?;

    tx.commit().await.map_err(|e| PortError::Unexpected {
        message: format!("Failed to commit tx for runtime migration: {}", e),
    })?;

    Ok(())
}
