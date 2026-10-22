use crate::sqlite::{SqliteProjectRepository, SqliteSpeechRunStore, connect_sqlite};
use domain::{
    job::{Job, JobKind},
    project::Project,
};
use ports::{
    repository::ProjectRepository,
    speech_run::{SpeechCheckpoint, SpeechRunIntent, SpeechRunStore},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn intent(project: &Project) -> SpeechRunIntent {
    let plan_json = r#"{"retained":"immutable legacy plan"}"#.to_owned();
    SpeechRunIntent {
        run_id: Uuid::new_v4(),
        project_id: project.id().clone(),
        plan_sha256: format!("{:x}", Sha256::digest(plan_json.as_bytes())),
        plan_json,
        profile_id: "legacy-profile".into(),
        profile_sha256: "a".repeat(64),
        expected_utterances: 1,
        expected_chunks: 2,
    }
}

#[tokio::test]
async fn version_17_rebuild_preserves_referencing_rows_and_all_foreign_keys()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("legacy.sqlite");
    let pool = crate::sqlite::connection::create_pool(&path).await?;
    sqlx::raw_sql(crate::sqlite::connection::SCHEMA)
        .execute(&pool)
        .await?;
    crate::sqlite::outbox_parallel_migration::migrate(&pool).await?;
    sqlx::raw_sql(concat!(
        include_str!("speech_workflow_schema.sql"),
        include_str!("speech_discard_schema.sql"),
        include_str!("puza_project_schema.sql")
    ))
    .execute(&pool)
    .await?;
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Legacy speech".into())?)
        .await?;
    let store = SqliteSpeechRunStore::new(pool.clone());
    let frozen = intent(&project);
    store.register_run(&frozen).await?;
    let attempt = Uuid::new_v4();
    let pending = store
        .create_pending(
            Job::new(project.id().clone(), "Pending".into(), JobKind::Speech),
            frozen.run_id,
            attempt,
        )
        .await?;
    let decision = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO speech_text_reviews VALUES (?, ?, '{\"keep\":true}', 'now')")
        .bind(&decision)
        .bind(project.id().to_string())
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO speech_run_reviews VALUES (?, ?)")
        .bind(frozen.run_id.to_string())
        .bind(&decision)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO speech_publications VALUES (?, ?, '{\"keep\":true}', 0, 'now')")
        .bind(frozen.run_id.to_string())
        .bind(attempt.to_string())
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO speech_host_checkpoints VALUES (?, 1, 'u1', 'cue-1', 'chunk1', ?, ?, 100, 'generated', 'now')")
        .bind(attempt.to_string()).bind("b".repeat(64)).bind("c".repeat(64)).execute(&pool).await?;
    pool.close().await;

    for _ in 0..2 {
        let reopened = connect_sqlite(&path).await?;
        let store = SqliteSpeechRunStore::new(reopened.clone());
        assert_eq!(store.get_run(frozen.run_id).await?, Some(frozen.clone()));
        assert_eq!(
            store
                .active_for_run(frozen.run_id)
                .await?
                .ok_or("lost attempt")?
                .job,
            pending
        );
        for (table, sql) in [
            (
                "speech_host_checkpoints",
                "SELECT COUNT(*) FROM speech_host_checkpoints",
            ),
            (
                "speech_run_reviews",
                "SELECT COUNT(*) FROM speech_run_reviews",
            ),
            (
                "speech_publications",
                "SELECT COUNT(*) FROM speech_publications",
            ),
            (
                "speech_text_reviews",
                "SELECT COUNT(*) FROM speech_text_reviews",
            ),
        ] {
            let count: i64 = sqlx::query_scalar(sql).fetch_one(&reopened).await?;
            assert_eq!(count, 1, "lost rows in {table}");
        }
        let review: String = sqlx::query_scalar("SELECT snapshot_json FROM speech_text_reviews")
            .fetch_one(&reopened)
            .await?;
        assert_eq!(review, r#"{"keep":true}"#);
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&reopened)
                .await?
                .is_empty()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
                .fetch_one(&reopened)
                .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("PRAGMA user_version")
                .fetch_one(&reopened)
                .await?,
            18
        );
        reopened.close().await;
    }
    Ok(())
}

#[tokio::test]
async fn failed_foreign_key_validation_rolls_back_tables_data_and_version()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("broken-legacy.sqlite");
    let pool = crate::sqlite::connection::create_pool(&path).await?;
    sqlx::raw_sql(crate::sqlite::connection::SCHEMA)
        .execute(&pool)
        .await?;
    crate::sqlite::outbox_parallel_migration::migrate(&pool).await?;
    sqlx::raw_sql(concat!(
        include_str!("speech_workflow_schema.sql"),
        include_str!("speech_discard_schema.sql"),
        include_str!("puza_project_schema.sql")
    ))
    .execute(&pool)
    .await?;
    let mut corrupt = pool.acquire().await?.detach();
    use sqlx::Connection;
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&mut corrupt)
        .await?;
    sqlx::query("INSERT INTO speech_run_reviews VALUES ('missing-run', 'missing-review')")
        .execute(&mut corrupt)
        .await?;
    corrupt.close().await?;
    pool.close().await;
    assert!(connect_sqlite(&path).await.is_err());
    let unchanged = crate::sqlite::connection::create_pool(&path).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA user_version")
            .fetch_one(&unchanged)
            .await?,
        17
    );
    let schema: String =
        sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE name='speech_run_intents'")
            .fetch_one(&unchanged)
            .await?;
    assert!(schema.contains("BETWEEN 1 AND 512"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM speech_run_reviews")
            .fetch_one(&unchanged)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name LIKE '%_v18'")
            .fetch_one(&unchanged)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
            .fetch_one(&unchanged)
            .await?,
        1
    );
    unchanged.close().await;
    Ok(())
}

#[tokio::test]
async fn long_intents_and_checkpoint_4097_persist_but_hard_limits_remain_enforced()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("long.sqlite");
    let pool = connect_sqlite(&path).await?;
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Long speech".into())?)
        .await?;
    let store = SqliteSpeechRunStore::new(pool.clone());
    let mut frozen = intent(&project);
    frozen.expected_utterances = 600;
    frozen.expected_chunks = 5000;
    store.register_run(&frozen).await?;
    let attempt = Uuid::new_v4();
    let pending = store
        .create_pending(
            Job::new(project.id().clone(), "Long".into(), JobKind::Speech),
            frozen.run_id,
            attempt,
        )
        .await?;
    let mut running = pending.clone();
    running.start()?;
    store.start(&running, attempt, pending.revision()).await?;
    sqlx::query("WITH RECURSIVE seq(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM seq WHERE n<4096) INSERT INTO speech_host_checkpoints SELECT ?, n, 'u1', 'cue-1', 'chunk-'||n, ?, ?, 100, 'generated', 'now' FROM seq")
        .bind(attempt.to_string()).bind("b".repeat(64)).bind("c".repeat(64)).execute(&pool).await?;
    let mut checkpointed = running.clone();
    checkpointed.report_speech_progress(4097, 5000)?;
    store
        .record_checkpoint(
            &checkpointed,
            &SpeechCheckpoint {
                run_id: frozen.run_id,
                attempt_id: attempt,
                plan_sha256: frozen.plan_sha256.clone(),
                sequence: 4097,
                utterance_id: "u1".into(),
                cue_id: Some("cue-1".into()),
                chunk_id: "chunk-4097".into(),
                cache_key: "b".repeat(64),
                audio_sha256: "c".repeat(64),
                duration_ms: 100,
                origin: "cached".into(),
            },
            running.revision(),
        )
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM speech_host_checkpoints")
            .fetch_one(&pool)
            .await?,
        4097
    );
    let mut oversized = frozen.clone();
    oversized.run_id = Uuid::new_v4();
    oversized.expected_utterances = 8193;
    oversized.expected_chunks = 8193;
    assert!(store.register_run(&oversized).await.is_err());
    assert!(
        sqlx::query("UPDATE speech_run_intents SET expected_chunks=8193")
            .execute(&pool)
            .await
            .is_err()
    );
    pool.close().await;
    let reopened = connect_sqlite(&path).await?;
    let store = SqliteSpeechRunStore::new(reopened.clone());
    assert_eq!(store.get_run(frozen.run_id).await?, Some(frozen.clone()));
    assert_eq!(
        store
            .active_for_run(frozen.run_id)
            .await?
            .ok_or("lost progress")?
            .job,
        checkpointed
    );
    reopened.close().await;
    Ok(())
}
