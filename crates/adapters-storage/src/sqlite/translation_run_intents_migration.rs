use ports::error::PortError;
use sqlx::SqlitePool;

use super::helpers::map_sqlite_error;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| map_sqlite_error("begin_translation_run_intents_migration", error))?;
    sqlx::raw_sql(include_str!("translation_run_intents_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("migrate_translation_run_intents", error))?;
    transaction
        .commit()
        .await
        .map_err(|error| map_sqlite_error("commit_translation_run_intents_migration", error))
}
