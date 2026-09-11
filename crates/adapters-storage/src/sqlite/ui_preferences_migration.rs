use ports::error::PortError;
use sqlx::SqlitePool;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool.begin().await.map_err(map_error)?;
    sqlx::raw_sql(include_str!("ui_preferences_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(map_error)?;
    transaction.commit().await.map_err(map_error)
}

fn map_error(error: sqlx::Error) -> PortError {
    super::helpers::map_sqlite_error("migrate_ui_preferences", error)
}
