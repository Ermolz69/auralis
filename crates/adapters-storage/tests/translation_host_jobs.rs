use adapters_storage::sqlite::recovery::SqliteRecoveryStorage;
use adapters_storage::sqlite::{
    SqliteJobRepository, SqliteProjectRepository, SqliteTranslationHostJobStore, connect_sqlite,
};
use domain::job::{Job, JobKind};
use domain::project::Project;
use ports::error::PortError;
use ports::recovery::RecoveryStorage;
use ports::repository::{JobRepository, ProjectRepository};
use ports::translation_host_job::TranslationHostJobStore;
use sqlx::SqlitePool;
use std::error::Error;
use uuid::Uuid;

async fn seed_run(pool: &SqlitePool) -> Result<(Project, Uuid, Uuid), Box<dyn Error>> {
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Translation".into())?)
        .await?;
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES ('source-1', ?, 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now')")
        .bind(project.id().to_string())
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO project_translations (translation_id, project_id, source_artifact_id, source_sha256, source_format, source_language, target_language, active_run_id, created_at, updated_at) VALUES (?, ?, 'source-1', ?, 'srt', 'zh', 'ru', ?, 'now', 'now')")
        .bind(translation_id.to_string())
        .bind(project.id().to_string())
        .bind("a".repeat(64))
        .bind(run_id.to_string())
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO translation_run_intents (run_id, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at) VALUES (?, ?, '{}', ?, 1, 0, 0, 1, ?, 'now')")
        .bind(run_id.to_string())
        .bind(translation_id.to_string())
        .bind("b".repeat(64))
        .bind("c".repeat(64))
        .execute(pool)
        .await?;
    Ok((project, translation_id, run_id))
}

#[tokio::test]
async fn job_and_run_association_commit_together_and_reject_concurrent_start()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let (project, translation_id, run_id) = seed_run(&pool).await?;
    let store = SqliteTranslationHostJobStore::new(pool.clone());
    let first = store
        .create_pending(
            Job::new(project.id().clone(), "First".into(), JobKind::Translation),
            translation_id,
            run_id,
        )
        .await?;
    assert_eq!(store.active_for_run(run_id).await?, Some(first.clone()));
    assert!(
        matches!(
            store
                .create_pending(
                    Job::new(project.id().clone(), "Second".into(), JobKind::Translation,),
                    translation_id,
                    run_id,
                )
                .await,
            Err(PortError::Conflict { .. })
        ),
        "a second active job must fail"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1, "failed association must roll back its job");
    assert_eq!(
        SqliteJobRepository::new(pool.clone())
            .get(first.id())
            .await?,
        Some(first.clone())
    );
    let mut stopped = first.clone();
    stopped.cancel()?;
    store.finish(&stopped, first.revision()).await?;
    assert!(store.active_for_run(run_id).await?.is_none());
    let resumed = store
        .create_pending(
            Job::new(project.id().clone(), "Resumed".into(), JobKind::Translation),
            translation_id,
            run_id,
        )
        .await?;
    assert_eq!(store.active_for_run(run_id).await?, Some(resumed.clone()));
    let active = store.list_active().await?;
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].run_id, run_id);
    assert_eq!(active[0].job, resumed);
    let mut running = resumed.clone();
    running.start()?;
    store.start(&running, resumed.revision()).await?;
    assert!(matches!(
        store.start(&running, resumed.revision()).await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(store.active_for_run(run_id).await?, Some(running.clone()));
    let mut checkpointed = running.clone();
    checkpointed.report_translation_progress(1, 2)?;
    store
        .record_progress(&checkpointed, running.revision())
        .await?;
    assert_eq!(
        store
            .active_for_run(run_id)
            .await?
            .ok_or("missing running job")?
            .progress()
            .processed_items,
        Some(1)
    );
    assert!(checkpointed.report_translation_progress(0, 2).is_err());
    assert!(matches!(
        store
            .record_progress(&checkpointed, running.revision())
            .await,
        Err(PortError::Conflict { .. })
    ));
    let dubbing_recovery = SqliteRecoveryStorage::new(pool.clone())
        .load_snapshot()
        .await?;
    assert!(dubbing_recovery.active_jobs.is_empty());
    let history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM translation_host_jobs WHERE run_id = ?")
            .bind(run_id.to_string())
            .fetch_one(&pool)
            .await?;
    assert_eq!(history_count, 2);
    Ok(())
}

#[tokio::test]
async fn job_creation_requires_matching_project_active_run_and_translation_kind()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let (project, translation_id, run_id) = seed_run(&pool).await?;
    let other = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Other".into())?)
        .await?;
    let store = SqliteTranslationHostJobStore::new(pool.clone());
    assert!(matches!(
        store
            .create_pending(
                Job::new(
                    other.id().clone(),
                    "Wrong project".into(),
                    JobKind::Translation
                ),
                translation_id,
                run_id,
            )
            .await,
        Err(PortError::Conflict { .. })
    ));
    assert!(matches!(
        store
            .create_pending(
                Job::new(project.id().clone(), "Wrong kind".into(), JobKind::Dubbing),
                translation_id,
                run_id,
            )
            .await,
        Err(PortError::InvalidSource { .. })
    ));
    sqlx::query("UPDATE project_translations SET active_run_id = NULL WHERE translation_id = ?")
        .bind(translation_id.to_string())
        .execute(&pool)
        .await?;
    assert!(matches!(
        store
            .create_pending(
                Job::new(project.id().clone(), "Stale".into(), JobKind::Translation),
                translation_id,
                run_id,
            )
            .await,
        Err(PortError::Conflict { .. })
    ));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn stale_terminal_update_keeps_the_association_active() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let (project, translation_id, run_id) = seed_run(&pool).await?;
    let store = SqliteTranslationHostJobStore::new(pool.clone());
    let job = store
        .create_pending(
            Job::new(project.id().clone(), "First".into(), JobKind::Translation),
            translation_id,
            run_id,
        )
        .await?;
    let mut stopped = job.clone();
    stopped.cancel()?;
    sqlx::query("UPDATE jobs SET revision = revision + 1 WHERE id = ?")
        .bind(job.id().to_string())
        .execute(&pool)
        .await?;
    assert!(matches!(
        store.finish(&stopped, job.revision()).await,
        Err(PortError::Conflict { .. })
    ));
    let active = store.active_for_run(run_id).await?.ok_or("missing job")?;
    assert_eq!(active.id(), job.id());
    assert_eq!(active.status(), job.status());
    let ended_at: Option<String> =
        sqlx::query_scalar("SELECT ended_at FROM translation_host_jobs WHERE job_id = ?")
            .bind(job.id().to_string())
            .fetch_one(&pool)
            .await?;
    assert!(ended_at.is_none());
    Ok(())
}
