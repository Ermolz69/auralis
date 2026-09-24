use ports::error::PortError;
use sqlx::SqlitePool;

use super::helpers::map_sqlite_error;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| map_sqlite_error("begin_translation_host_jobs_migration", error))?;
    sqlx::raw_sql(include_str!("translation_host_jobs_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("migrate_translation_host_jobs", error))?;
    transaction
        .commit()
        .await
        .map_err(|error| map_sqlite_error("commit_translation_host_jobs_migration", error))
}
