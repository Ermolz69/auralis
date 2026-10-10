mod admission;
mod dispatch;
mod mapping;
mod repository;

pub use repository::SqliteJobExecutionStore;

pub(crate) async fn migrate(pool: &sqlx::SqlitePool) -> Result<(), ports::PortError> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| super::helpers::map_sqlite_error("begin_job_queue_migration", e))?;
    sqlx::raw_sql(include_str!("../job_executions_schema.sql"))
        .execute(&mut *tx)
        .await
        .map_err(|e| super::helpers::map_sqlite_error("job_queue_migration", e))?;
    tx.commit()
        .await
        .map_err(|e| super::helpers::map_sqlite_error("commit_job_queue_migration", e))
}
