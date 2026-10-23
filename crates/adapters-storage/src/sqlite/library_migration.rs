use ports::error::PortError;
use sqlx::SqlitePool;
pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool.begin().await.map_err(error)?;
    sqlx::raw_sql(include_str!("library_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(error)?;
    sqlx::query("PRAGMA user_version = 18")
        .execute(&mut *transaction)
        .await
        .map_err(error)?;
    transaction.commit().await.map_err(error)
}
fn error(e: sqlx::Error) -> PortError {
    super::helpers::map_sqlite_error("migrate_workspace_library", e)
}
