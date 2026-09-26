use super::helpers::map_sqlite_error;
use ports::error::PortError;
use sqlx::SqlitePool;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| map_sqlite_error("begin_translation_edit_intents_migration", error))?;
    sqlx::raw_sql(include_str!("translation_edit_intents_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("migrate_translation_edit_intents", error))?;
    transaction
        .commit()
        .await
        .map_err(|error| map_sqlite_error("commit_translation_edit_intents_migration", error))
}
