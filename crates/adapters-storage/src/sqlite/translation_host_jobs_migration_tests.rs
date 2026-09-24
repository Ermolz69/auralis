use super::connection::{connect_sqlite, create_pool};
use super::job_repository::SqliteJobRepository;
use super::project_repository::SqliteProjectRepository;
use domain::job::{Job, JobKind};
use domain::project::Project;
use ports::repository::{JobRepository, ProjectRepository};
use std::error::Error;

#[tokio::test]
async fn v7_database_gains_one_active_host_job_per_run_without_losing_link()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("migration.sqlite");
    let pool = create_pool(&path).await?;
    let old_schema = concat!(
        include_str!("schema.sql"),
        include_str!("youtube_import_schema.sql"),
        include_str!("ui_preferences_schema.sql"),
        include_str!("translation_links_schema.sql"),
        include_str!("translation_run_intents_schema.sql")
    );
    sqlx::raw_sql(old_schema).execute(&pool).await?;
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Preserved".into())?)
        .await?;
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES ('source-1', ?, 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now')")
        .bind(project.id().to_string())
        .execute(&pool).await?;
    sqlx::query("INSERT INTO project_translations (translation_id, project_id, source_artifact_id, source_sha256, source_format, source_language, target_language, active_run_id, created_at, updated_at) VALUES ('translation-1', ?, 'source-1', ?, 'srt', 'zh', 'ru', 'run-1', 'now', 'now')")
        .bind(project.id().to_string())
        .bind("a".repeat(64))
        .execute(&pool).await?;
    sqlx::query("INSERT INTO translation_run_intents (run_id, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at) VALUES ('run-1', 'translation-1', '{}', ?, 1, 0, 0, 1, ?, 'now')")
        .bind("b".repeat(64))
        .bind("c".repeat(64))
        .execute(&pool).await?;
    pool.close().await;

    let pool = connect_sqlite(&path).await?;
    let marker: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(marker, 8);
    let active_run: String = sqlx::query_scalar(
        "SELECT active_run_id FROM project_translations WHERE translation_id = 'translation-1'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(active_run, "run-1");

    let repository = SqliteJobRepository::new(pool.clone());
    let first = repository
        .create(Job::new(
            project.id().clone(),
            "First".into(),
            JobKind::Translation,
        ))
        .await?;
    let second = repository
        .create(Job::new(
            project.id().clone(),
            "Second".into(),
            JobKind::Translation,
        ))
        .await?;
    sqlx::query("INSERT INTO translation_host_jobs (job_id, run_id, created_at) VALUES (?, 'run-1', 'first')")
        .bind(first.id().to_string())
        .execute(&pool).await?;
    assert!(
        sqlx::query("INSERT INTO translation_host_jobs (job_id, run_id, created_at) VALUES (?, 'run-1', 'second')")
            .bind(second.id().to_string())
            .execute(&pool).await.is_err()
    );
    sqlx::query("UPDATE translation_host_jobs SET ended_at = 'ended' WHERE job_id = ?")
        .bind(first.id().to_string())
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO translation_host_jobs (job_id, run_id, created_at) VALUES (?, 'run-1', 'second')")
        .bind(second.id().to_string())
        .execute(&pool).await?;
    pool.close().await;

    let reopened = connect_sqlite(&path).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM translation_host_jobs WHERE run_id = 'run-1'")
            .fetch_one(&reopened)
            .await?;
    assert_eq!(count, 2);
    Ok(())
}
