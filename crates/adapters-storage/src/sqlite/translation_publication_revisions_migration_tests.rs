use super::connection::{connect_sqlite, create_pool};
use std::error::Error;

#[tokio::test]
async fn v8_publications_gain_result_revision_without_losing_pending_work()
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
        include_str!("translation_host_jobs_schema.sql")
    ))
    .execute(&pool)
    .await?;
    sqlx::query("ALTER TABLE translation_publications DROP COLUMN result_revision")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA user_version = 8")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES ('project-1', 'Preserved', 'Draft', 'now', 'now')")
        .execute(&pool).await?;
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES ('source-1', 'project-1', 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now'), ('output-1', 'project-1', 'TranslatedTranscript', 'output.srt', 'pending_finalize', 'now', 'now')")
        .execute(&pool).await?;
    sqlx::query("INSERT INTO project_translations (translation_id, project_id, source_artifact_id, source_sha256, source_format, source_language, target_language, active_run_id, created_at, updated_at) VALUES ('translation-1', 'project-1', 'source-1', ?, 'srt', 'zh', 'ru', 'run-1', 'now', 'now')")
        .bind("a".repeat(64)).execute(&pool).await?;
    sqlx::query("INSERT INTO translation_publications (translation_id, result_id, run_id, artifact_id, source_sha256, expected_link_revision, state, needs_review, created_at, updated_at) VALUES ('translation-1', 'result-1', 'run-1', 'output-1', ?, 1, 'pending', 1, 'now', 'now')")
        .bind("a".repeat(64)).execute(&pool).await?;
    pool.close().await;

    let migrated = connect_sqlite(&path).await?;
    let (revision, state): (i64, String) = sqlx::query_as(
        "SELECT result_revision, state FROM translation_publications WHERE result_id = 'result-1'",
    )
    .fetch_one(&migrated)
    .await?;
    assert_eq!((revision, state.as_str()), (1, "pending"));
    let marker: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&migrated)
        .await?;
    assert_eq!(marker, 16);
    migrated.close().await;
    let reopened = connect_sqlite(&path).await?;
    let revision: i64 = sqlx::query_scalar(
        "SELECT result_revision FROM translation_publications WHERE result_id = 'result-1'",
    )
    .fetch_one(&reopened)
    .await?;
    assert_eq!(revision, 1);
    Ok(())
}
