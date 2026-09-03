use super::helpers::map_sqlite_error;
use ports::error::PortError;
use sqlx::SqlitePool;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| map_sqlite_error("migrate_youtube_imports", e))?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| map_sqlite_error("migrate_youtube_imports", e))?;
    if version == 3 {
        sqlx::raw_sql(include_str!("youtube_import_schema.sql"))
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlite_error("migrate_youtube_imports", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| map_sqlite_error("migrate_youtube_imports", e))
}
