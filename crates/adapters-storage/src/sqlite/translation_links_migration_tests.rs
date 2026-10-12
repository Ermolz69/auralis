#![allow(clippy::unwrap_used)]

use super::connection::{connect_sqlite, create_pool};

#[tokio::test]
async fn existing_v5_database_gains_links_without_losing_projects() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("migration.sqlite");
    let pool = create_pool(&path).await.unwrap();
    let old_schema = concat!(
        include_str!("schema.sql"),
        include_str!("youtube_import_schema.sql"),
        include_str!("ui_preferences_schema.sql")
    );
    sqlx::raw_sql(old_schema).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES ('project-1', 'Preserved', 'draft', 'now', 'now')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let pool = connect_sqlite(&path).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .unwrap(),
        16
    );
    let title: String = sqlx::query_scalar("SELECT title FROM projects WHERE id = 'project-1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "Preserved");
    pool.close().await;

    let reopened = connect_sqlite(&path).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_translations")
        .fetch_one(&reopened)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn link_schema_requires_existing_project_and_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(directory.path().join("links.sqlite"))
        .await
        .unwrap();
    let insert = "INSERT INTO project_translations (
        translation_id, project_id, source_artifact_id, source_sha256,
        source_format, source_language, target_language, active_run_id,
        created_at, updated_at
    ) VALUES ('translation-1', ?, ?, ?, 'srt', 'zh', 'ru', 'run-1', 'now', 'now')";
    let hash = "a".repeat(64);
    assert!(
        sqlx::query(insert)
            .bind("missing")
            .bind("source-1")
            .bind(&hash)
            .execute(&pool)
            .await
            .is_err()
    );
    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES ('project-1', 'Test', 'draft', 'now', 'now')",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        sqlx::query(insert)
            .bind("project-1")
            .bind("missing")
            .bind(&hash)
            .execute(&pool)
            .await
            .is_err()
    );
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, created_at, updated_at) VALUES ('source-1', 'project-1', 'OriginalSubtitle', 'source-1.srt', 'now', 'now')")
        .execute(&pool).await.unwrap();
    sqlx::query(insert)
        .bind("project-1")
        .bind("source-1")
        .bind(&hash)
        .execute(&pool)
        .await
        .unwrap();
    assert!(sqlx::query("UPDATE project_translations SET selected_result_id = 'result-1' WHERE translation_id = 'translation-1'")
        .execute(&pool).await.is_err());
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, created_at, updated_at) VALUES ('output-1', 'project-1', 'TranslatedTranscript', 'output-1.srt', 'now', 'now')")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO translation_publications (translation_id, result_id, run_id, artifact_id, source_sha256, expected_link_revision, state, needs_review, created_at, updated_at) VALUES ('translation-1', 'result-1', 'run-1', 'output-1', ?, 1, 'pending', 1, 'now', 'now')")
        .bind(&hash).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM projects WHERE id = 'project-1'")
        .execute(&pool)
        .await
        .unwrap();
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_translations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
}
