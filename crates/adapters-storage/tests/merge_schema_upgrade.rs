#[path = "support/legacy_outbox.rs"]
mod legacy_outbox;

use adapters_storage::sqlite::connect_sqlite;
use legacy_outbox::restore_legacy_outbox_shape;
use sqlx::{SqlitePool, query, raw_sql};
use std::error::Error;

async fn version(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
}

async fn table_count(pool: &SqlitePool, name: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
        .bind(name)
        .fetch_one(pool)
        .await
}

#[tokio::test]
async fn review_only_v11_reaches_combined_schema() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("review-only.sqlite");
    let pool = connect_sqlite(&database).await?;
    restore_legacy_outbox_shape(&pool).await?;
    raw_sql("DROP TABLE job_executions; DROP TABLE speech_host_checkpoints; DROP TABLE speech_host_attempts; DROP TABLE speech_run_intents; PRAGMA user_version = 11;")
        .execute(&pool)
        .await?;
    pool.close().await;

    let upgraded = connect_sqlite(&database).await?;
    assert_eq!(version(&upgraded).await?, 16);
    assert_eq!(
        table_count(&upgraded, "translation_review_events").await?,
        1
    );
    assert_eq!(table_count(&upgraded, "speech_run_intents").await?, 1);
    assert_eq!(table_count(&upgraded, "job_executions").await?, 1);
    upgraded.close().await;
    let reopened = connect_sqlite(&database).await?;
    assert_eq!(version(&reopened).await?, 16);
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn speech_queue_v12_reaches_combined_schema() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("speech-queue.sqlite");
    let pool = connect_sqlite(&database).await?;
    restore_legacy_outbox_shape(&pool).await?;
    query("DROP TABLE translation_review_events")
        .execute(&pool)
        .await?;
    query("PRAGMA user_version = 12").execute(&pool).await?;
    pool.close().await;

    let upgraded = connect_sqlite(&database).await?;
    assert_eq!(version(&upgraded).await?, 16);
    assert_eq!(
        table_count(&upgraded, "translation_review_events").await?,
        1
    );
    assert_eq!(table_count(&upgraded, "speech_run_intents").await?, 1);
    assert_eq!(table_count(&upgraded, "job_executions").await?, 1);
    upgraded.close().await;
    Ok(())
}
