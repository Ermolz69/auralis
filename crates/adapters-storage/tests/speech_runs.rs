use adapters_storage::sqlite::{SqliteProjectRepository, SqliteSpeechRunStore, connect_sqlite};
use domain::job::{Job, JobKind};
use domain::project::Project;
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::speech_run::{SpeechCheckpoint, SpeechRunIntent, SpeechRunStore};
use sha2::{Digest, Sha256};
use std::error::Error;
use uuid::Uuid;

fn intent(project: &Project) -> SpeechRunIntent {
    let plan_json = r#"{"schema_version":1,"utterances":[]}"#.to_owned();
    SpeechRunIntent {
        run_id: Uuid::new_v4(),
        project_id: project.id().clone(),
        plan_sha256: format!("{:x}", Sha256::digest(plan_json.as_bytes())),
        plan_json,
        profile_id: "native-plan-voxcpm2".into(),
        profile_sha256: "a".repeat(64),
        expected_utterances: 1,
        expected_chunks: 2,
    }
}

fn checkpoint(intent: &SpeechRunIntent, attempt_id: Uuid, sequence: u32) -> SpeechCheckpoint {
    SpeechCheckpoint {
        run_id: intent.run_id,
        attempt_id,
        plan_sha256: intent.plan_sha256.clone(),
        sequence,
        utterance_id: "u1".into(),
        cue_id: Some("cue1".into()),
        chunk_id: format!("chunk{sequence}"),
        cache_key: "b".repeat(64),
        audio_sha256: "c".repeat(64),
        duration_ms: 500,
        origin: "generated".into(),
    }
}

#[tokio::test]
async fn speech_attempts_persist_progress_and_fence_late_events() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Speech".into())?)
        .await?;
    let store = SqliteSpeechRunStore::new(pool.clone());
    let frozen = intent(&project);
    store.register_run(&frozen).await?;
    store.register_run(&frozen).await?;
    assert_eq!(store.get_run(frozen.run_id).await?, Some(frozen.clone()));
    let mut changed = frozen.clone();
    changed.profile_id = "different-profile".into();
    assert!(matches!(
        store.register_run(&changed).await,
        Err(PortError::Conflict { .. })
    ));

    let first_attempt = Uuid::new_v4();
    let pending = store
        .create_pending(
            Job::new(project.id().clone(), "Speech".into(), JobKind::Speech),
            frozen.run_id,
            first_attempt,
        )
        .await?;
    assert!(matches!(
        store
            .create_pending(
                Job::new(project.id().clone(), "Duplicate".into(), JobKind::Speech),
                frozen.run_id,
                Uuid::new_v4(),
            )
            .await,
        Err(PortError::Conflict { .. })
    ));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1, "rejected admission rolls back its job");

    let mut running = pending.clone();
    running.start()?;
    store
        .start(&running, first_attempt, pending.revision())
        .await?;
    let mut checkpointed = running.clone();
    checkpointed.report_speech_progress(1, 2)?;
    let saved = checkpoint(&frozen, first_attempt, 1);
    store
        .record_checkpoint(&checkpointed, &saved, running.revision())
        .await?;
    assert!(matches!(
        store
            .record_checkpoint(&checkpointed, &saved, running.revision())
            .await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(
        store
            .active_for_run(frozen.run_id)
            .await?
            .ok_or("missing attempt")?
            .job,
        checkpointed
    );
    let saved_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM speech_host_checkpoints WHERE attempt_id = ?")
            .bind(first_attempt.to_string())
            .fetch_one(&pool)
            .await?;
    assert_eq!(saved_count, 1);

    let mut stopped = checkpointed.clone();
    stopped.cancel()?;
    store
        .finish(&stopped, first_attempt, checkpointed.revision())
        .await?;
    assert!(store.active_for_run(frozen.run_id).await?.is_none());
    let resumed_attempt = Uuid::new_v4();
    let resumed = store
        .create_pending(
            Job::new(
                project.id().clone(),
                "Resume speech".into(),
                JobKind::Speech,
            ),
            frozen.run_id,
            resumed_attempt,
        )
        .await?;
    assert_eq!(store.list_active().await?.len(), 1);
    let mut resumed_running = resumed.clone();
    resumed_running.start()?;
    store
        .start(&resumed_running, resumed_attempt, resumed.revision())
        .await?;
    assert!(matches!(
        store
            .record_checkpoint(&checkpointed, &saved, running.revision())
            .await,
        Err(PortError::Conflict { .. })
    ));
    assert!(matches!(
        store
            .finish(&stopped, first_attempt, checkpointed.revision())
            .await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(
        store
            .active_for_run(frozen.run_id)
            .await?
            .ok_or("missing resumed attempt")?
            .attempt_id,
        resumed_attempt
    );
    Ok(())
}
