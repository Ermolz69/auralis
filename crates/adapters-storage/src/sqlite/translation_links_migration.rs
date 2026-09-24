use ports::error::PortError;
use sqlx::SqlitePool;

use super::helpers::map_sqlite_error;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool.begin().await.map_err(map_error)?;
    sqlx::raw_sql(include_str!("translation_links_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(map_error)?;
    transaction.commit().await.map_err(map_error)
}

fn map_error(error: sqlx::Error) -> PortError {
    map_sqlite_error("migrate_translation_links", error)
}
