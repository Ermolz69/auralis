use adapters_storage::{
    local::artifact_store::LocalArtifactStore,
    sqlite::{SqliteArtifactIndex, SqliteProjectRepository, SqliteSpeechRunStore, connect_sqlite},
};
use domain::{
    job::{Job, JobKind, JobStatus},
    media::ArtifactKind,
    project::Project,
};
use ports::{
    artifact_index::ArtifactIndex,
    repository::ProjectRepository,
    speech_run::{SpeechCheckpoint, SpeechRunIntent, SpeechRunStore},
    speech_workflow::{
        SpeechPublication, SpeechPublishedFile, SpeechReview, SpeechSourceSnapshot,
        SpeechWorkflowStore,
    },
    storage::ArtifactStore,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[tokio::test]
async fn publication_is_atomic_fenced_and_hidden_until_every_file_is_ready()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let pool = connect_sqlite(dir.path().join("db.sqlite")).await?;
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Speech publication".into())?)
        .await?;
    let store = SqliteSpeechRunStore::new(pool.clone());
    let index = SqliteArtifactIndex::new(pool.clone());
    let artifacts = LocalArtifactStore::new(dir.path().join("projects"));
    let digest = format!("{:x}", Sha256::digest(b"Fixture text"));
    let review = SpeechReview {
        original_source: None,
        decision_id: Uuid::new_v4(),
        project_id: project.id().clone(),
        source: SpeechSourceSnapshot {
            kind: "text".into(),
            source_id: None,
            revision: digest.clone(),
            sha256: digest.clone(),
            text: "Fixture text".into(),
            cues: vec![],
        },
    };
    store.save_review(&review).await?;
    assert_eq!(
        store.get_review(review.decision_id).await?,
        Some(review.clone())
    );
    let plan_json = serde_json::json!({"review":{"decision_id":review.decision_id},"source":{"source_sha256":digest}}).to_string();
    let intent = SpeechRunIntent {
        run_id: Uuid::new_v4(),
        project_id: project.id().clone(),
        plan_sha256: format!("{:x}", Sha256::digest(plan_json.as_bytes())),
        plan_json,
        profile_id: "native-plan-voxcpm2".into(),
        profile_sha256: "a".repeat(64),
        expected_utterances: 1,
        expected_chunks: 1,
    };
    assert!(
        store
            .register_reviewed_run(&intent, Uuid::new_v4())
            .await
            .is_err()
    );
    store
        .register_reviewed_run(&intent, review.decision_id)
        .await?;
    let attempt = Uuid::new_v4();
    let mut job = store
        .create_pending(
            Job::new(project.id().clone(), "Speech".into(), JobKind::Speech),
            intent.run_id,
            attempt,
        )
        .await?;
    let rev = job.revision();
    job.start()?;
    store.start(&job, attempt, rev).await?;
    let source = dir.path().join("fixture.wav");
    tokio::fs::write(&source, b"fixture bytes").await?;
    let mut staged = Vec::new();
    let mut files = Vec::new();
    for (kind, artifact_kind) in [
        ("wav", ArtifactKind::GeneratedSpeech),
        ("manifest", ArtifactKind::SpeechManifest),
        ("segment", ArtifactKind::GeneratedSpeechSegment),
    ] {
        let artifact = artifacts
            .import_external_file(
                project.id(),
                artifact_kind,
                &source,
                Some(if kind == "manifest" {
                    "manifest.json"
                } else {
                    "speech.wav"
                }),
            )
            .await?;
        files.push(SpeechPublishedFile {
            artifact_id: artifact.artifact.id.clone(),
            kind: kind.into(),
            sha256: format!("{:x}", Sha256::digest(b"fixture bytes")),
            size_bytes: artifact.size_bytes,
        });
        staged.push(artifact);
    }
    let publication = SpeechPublication {
        run_id: intent.run_id,
        attempt_id: attempt,
        duration_ms: 100,
        files,
        ready: false,
    };
    assert!(store.publish(&job, &publication, &staged).await.is_err());
    assert!(index.list_by_project(project.id()).await?.is_empty());
    let rev = job.revision();
    job.report_speech_progress(1, 1)?;
    store
        .record_checkpoint(
            &job,
            &SpeechCheckpoint {
                run_id: intent.run_id,
                attempt_id: attempt,
                plan_sha256: intent.plan_sha256.clone(),
                sequence: 1,
                utterance_id: "u1".into(),
                cue_id: None,
                chunk_id: "u1-chunk-0001".into(),
                cache_key: "b".repeat(64),
                audio_sha256: "c".repeat(64),
                duration_ms: 100,
                origin: "generated".into(),
            },
            rev,
        )
        .await?;
    let mut stale = publication.clone();
    stale.attempt_id = Uuid::new_v4();
    assert!(store.publish(&job, &stale, &staged).await.is_err());
    store.publish(&job, &publication, &staged).await?;
    assert!(store.publish(&job, &publication, &staged).await.is_err());
    let outbox: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages")
        .fetch_one(&pool)
        .await?;
    assert_eq!(outbox, 3);
    for (q, file) in publication.files.iter().enumerate() {
        sqlx::query("UPDATE artifacts SET state='ready' WHERE id=?")
            .bind(file.artifact_id.to_string())
            .execute(&pool)
            .await?;
        assert!(index.get(&file.artifact_id).await?.is_none());
        if q < 2 {
            assert!(store.complete_publication(intent.run_id).await?.is_none());
        }
    }
    pool.close().await;
    let reopened = connect_sqlite(dir.path().join("db.sqlite")).await?;
    let store = SqliteSpeechRunStore::new(reopened.clone());
    let completed = store
        .complete_publication(intent.run_id)
        .await?
        .ok_or("Publication did not recover")?;
    assert_eq!(completed.status(), &JobStatus::Completed);
    assert!(store.active_for_run(intent.run_id).await?.is_none());
    assert!(
        store
            .publication(intent.run_id)
            .await?
            .ok_or("Missing publication")?
            .ready
    );
    assert_eq!(
        SqliteArtifactIndex::new(reopened)
            .list_by_project(project.id())
            .await?
            .len(),
        3
    );
    Ok(())
}
