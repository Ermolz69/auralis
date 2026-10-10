use super::connection::{connect_sqlite, create_pool};
use std::error::Error;

#[tokio::test]
async fn v10_database_gains_speech_tables_without_losing_existing_jobs()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("migration.sqlite");
    let pool = create_pool(&path).await?;
    sqlx::raw_sql(concat!(
        include_str!("schema.sql"),
        include_str!("youtube_import_schema.sql"),
        include_str!("ui_preferences_schema.sql"),
        include_str!("translation_links_schema.sql"),
        include_str!("translation_run_intents_schema.sql"),
        include_str!("translation_host_jobs_schema.sql"),
        include_str!("translation_publication_revisions_schema.sql"),
        include_str!("translation_edit_intents_schema.sql")
    ))
    .execute(&pool)
    .await?;
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES ('existing', 'Keep', 'Draft', 'now', 'now')")
        .execute(&pool).await?;
    pool.close().await;

    let migrated = connect_sqlite(&path).await?;
    let marker: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&migrated)
        .await?;
    assert_eq!(marker, 11);
    let title: String = sqlx::query_scalar("SELECT title FROM projects WHERE id = 'existing'")
        .fetch_one(&migrated)
        .await?;
    assert_eq!(title, "Keep");
    let speech_tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'speech_%'",
    )
    .fetch_one(&migrated)
    .await?;
    assert_eq!(speech_tables, 3);
    migrated.close().await;
    connect_sqlite(&path).await?;
    Ok(())
}
