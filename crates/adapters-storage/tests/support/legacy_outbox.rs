pub async fn restore_legacy_outbox_shape(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "DROP INDEX idx_outbox_group_sequence;
         ALTER TABLE outbox_messages DROP COLUMN sequence;
         ALTER TABLE outbox_messages DROP COLUMN conflict_group;
         DROP TABLE outbox_sequence;",
    )
    .execute(pool)
    .await?;
    Ok(())
}
