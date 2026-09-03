use ports::error::PortError;
use sqlx::SqlitePool;

use super::helpers::map_sqlite_error;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| map_sqlite_error("migrate_project_revision", e))?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| map_sqlite_error("migrate_project_revision", e))?;
    if version == 1 {
        sqlx::raw_sql(
            "ALTER TABLE projects ADD COLUMN revision INTEGER NOT NULL DEFAULT 1
             CHECK (typeof(revision) = 'integer' AND revision >= 1);
             PRAGMA user_version = 2;",
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| map_sqlite_error("migrate_project_revision", e))?;
    }
    tx.commit()
        .await
        .map_err(|e| map_sqlite_error("migrate_project_revision", e))
}
