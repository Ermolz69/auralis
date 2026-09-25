use super::helpers::map_sqlite_error;
use ports::error::PortError;
use sqlx::SqlitePool;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut transaction = pool.begin().await.map_err(|error| {
        map_sqlite_error("begin_translation_publication_revisions_migration", error)
    })?;
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM pragma_table_info('translation_publications') ORDER BY cid",
    )
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| map_sqlite_error("inspect_translation_publication_revisions", error))?;
    if !columns.iter().any(|name| name == "result_revision") {
        sqlx::query(
            "ALTER TABLE translation_publications ADD COLUMN result_revision INTEGER NOT NULL DEFAULT 1 CHECK (result_revision BETWEEN 1 AND 2147483647)",
        )
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("add_translation_result_revision", error))?;
    }
    sqlx::raw_sql(include_str!("translation_publication_revisions_schema.sql"))
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("migrate_translation_publication_revisions", error))?;
    transaction.commit().await.map_err(|error| {
        map_sqlite_error("commit_translation_publication_revisions_migration", error)
    })
}
