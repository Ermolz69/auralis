use super::helpers::map_sqlite_error;
use ports::error::PortError;
use sqlx::SqlitePool;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| map_sqlite_error("begin_speech_run_migration", error))?;
    sqlx::raw_sql(include_str!("speech_run_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("migrate_speech_runs", error))?;
    transaction
        .commit()
        .await
        .map_err(|error| map_sqlite_error("commit_speech_run_migration", error))
}
