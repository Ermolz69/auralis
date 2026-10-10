use ports::error::PortError;
use sqlx::Row;
use sqlx::SqlitePool;

use super::helpers::map_sqlite_error;

pub(super) async fn migrate(pool: &SqlitePool) -> Result<(), PortError> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| map_sqlite_error("begin_outbox_parallel_migration", error))?;
    sqlx::raw_sql(
        "ALTER TABLE outbox_messages ADD COLUMN sequence INTEGER;
         ALTER TABLE outbox_messages ADD COLUMN conflict_group TEXT;
         UPDATE outbox_messages SET sequence = rowid;
         UPDATE outbox_messages SET conflict_group = '*';
         CREATE TABLE outbox_sequence (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), value INTEGER NOT NULL);
         INSERT INTO outbox_sequence VALUES (1, COALESCE((SELECT MAX(sequence) FROM outbox_messages), 0));
         CREATE INDEX idx_outbox_group_sequence ON outbox_messages(conflict_group, sequence, status);
         PRAGMA user_version = 14;",
    )
    .execute(&mut *tx)
    .await
    .map_err(|error| map_sqlite_error("migrate_outbox_parallel", error))?;
    let rows = sqlx::query("SELECT id, aggregate_id, payload_json FROM outbox_messages WHERE aggregate_type = 'project'")
        .fetch_all(&mut *tx).await
        .map_err(|error| map_sqlite_error("read_legacy_outbox_groups", error))?;
    for row in rows {
        let (Ok(id), Ok(Some(aggregate_id)), Ok(payload_json)) = (
            row.try_get::<String, _>("id"),
            row.try_get::<Option<String>, _>("aggregate_id"),
            row.try_get::<String, _>("payload_json"),
        ) else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<domain::outbox::OutboxPayload>(&payload_json)
        else {
            continue;
        };
        let (aggregate_type, payload_project) = payload.aggregate_info();
        if aggregate_type.as_deref() == Some("project")
            && payload_project.as_deref() == Some(aggregate_id.as_str())
        {
            sqlx::query("UPDATE outbox_messages SET conflict_group = ? WHERE id = ?")
                .bind(format!("project:{aggregate_id}"))
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|error| map_sqlite_error("backfill_legacy_outbox_group", error))?;
        }
    }
    tx.commit()
        .await
        .map_err(|error| map_sqlite_error("commit_outbox_parallel_migration", error))
}
