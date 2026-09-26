use super::connection::{connect_sqlite, create_pool};
use std::error::Error;

#[tokio::test]
async fn existing_v6_links_gain_run_intents_without_losing_identity() -> Result<(), Box<dyn Error>>
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("migration.sqlite");
    let pool = create_pool(&path).await?;
    let old_schema = concat!(
        include_str!("schema.sql"),
        include_str!("youtube_import_schema.sql"),
        include_str!("ui_preferences_schema.sql"),
        include_str!("translation_links_schema.sql")
    );
    sqlx::raw_sql(old_schema).execute(&pool).await?;
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES ('project-1', 'Preserved', 'draft', 'now', 'now')")
        .execute(&pool).await?;
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES ('source-1', 'project-1', 'OriginalSubtitle', 'source-1.srt', 'ready', 'now', 'now')")
        .execute(&pool).await?;
    sqlx::query("INSERT INTO project_translations (translation_id, project_id, source_artifact_id, source_sha256, source_format, source_language, target_language, active_run_id, created_at, updated_at) VALUES ('translation-1', 'project-1', 'source-1', ?, 'srt', 'zh', 'ru', 'run-1', 'now', 'now')")
        .bind("a".repeat(64)).execute(&pool).await?;
    pool.close().await;

    let pool = connect_sqlite(&path).await?;
    let marker: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(marker, 10);
    let active: String = sqlx::query_scalar(
        "SELECT active_run_id FROM project_translations WHERE translation_id = 'translation-1'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(active, "run-1");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM translation_run_intents")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    pool.close().await;
    let reopened = connect_sqlite(&path).await?;
    let marker: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&reopened)
        .await?;
    assert_eq!(marker, 10);
    Ok(())
}
