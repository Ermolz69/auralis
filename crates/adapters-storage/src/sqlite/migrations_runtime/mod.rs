pub mod backfill_artifacts;

use ports::error::PortError;
use sqlx::SqlitePool;

pub async fn run_runtime_backfills(pool: &SqlitePool) -> Result<(), PortError> {
    backfill_artifacts::run(pool).await?;
    Ok(())
}
