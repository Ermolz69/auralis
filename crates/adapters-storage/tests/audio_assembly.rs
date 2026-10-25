#[allow(dead_code)]
#[path = "support/assembly_source.rs"]
mod source;
#[path = "support/video_publication.rs"]
mod video_publication;

use adapters_storage::sqlite::{
    SqliteArtifactIndex, SqliteAudioAssemblyStore, SqliteSpeechRunStore, SqliteStorageUnitOfWork,
    connect_sqlite,
};
use domain::{
    job::{Job, JobKind, JobStatus},
    media::ArtifactKind,
    outbox::OutboxMessageId,
    speech::Sha256Digest,
};
use ports::{
    artifact_finalization::{ArtifactFinalizationLookup, FinalizationMetadata},
    artifact_index::ArtifactIndex,
    audio_jobs::{AudioAssemblyIntent, AudioAssemblyPublication, AudioAssemblyStore},
    cancellation::CancellationToken,
    speech_workflow::{SpeechPublishedFile, SpeechWorkflowStore},
    storage::{ArtifactStore, StagedArtifact, VerifiedArtifactCopy},
    transaction::{CommitArtifactFinalize, CommitArtifactFinalizeResult, StorageUnitOfWork},
};
use source::AssemblySource;
use uuid::Uuid;

async fn running(
    fixture: &AssemblySource,
    store: &SqliteAudioAssemblyStore,
) -> Result<(AudioAssemblyIntent, Uuid, Job), Box<dyn std::error::Error>> {
    let intent = store.register(&fixture.intent()?).await?;
    let attempt = Uuid::new_v4();
    let mut job = store
        .create_pending(
            intent.assembly_id,
            attempt,
            Job::new(
                fixture.project_id.clone(),
                "Assembly fixture".into(),
                JobKind::AudioAssembly,
            ),
        )
        .await?;
    let revision = job.revision();
    job.start()?;
    store.update(&job, attempt, revision).await?;
    Ok((intent, attempt, job))
}

async fn output(
    fixture: &AssemblySource,
    intent: &AudioAssemblyIntent,
    attempt: Uuid,
) -> Result<(AudioAssemblyPublication, Vec<StagedArtifact>), Box<dyn std::error::Error>> {
    let wave = source::wave();
    let wave_path = fixture.directory.path().join("output.wav");
    std::fs::write(&wave_path, &wave)?;
    let manifest_path = fixture.directory.path().join("assembly.json");
    std::fs::write(&manifest_path, b"{\"fixture\":true}")?;
    let mut files = Vec::new();
    let mut staged = Vec::new();
    for (role, kind, path) in [
        ("wav", ArtifactKind::MixedAudio, wave_path),
        ("manifest", ArtifactKind::SpeechManifest, manifest_path),
    ] {
        let sha = source::digest(&std::fs::read(&path)?);
        let artifact = fixture
            .files
            .stage_verified_copy(
                VerifiedArtifactCopy {
                    project_id: fixture.project_id.clone(),
                    kind,
                    source_path: path,
                    filename_hint: None,
                    expected_sha256: Sha256Digest::parse(sha.clone())?,
                    max_bytes: 1024 * 1024,
                },
                CancellationToken::new(),
            )
            .await?;
        files.push(SpeechPublishedFile {
            artifact_id: artifact.artifact.id.clone(),
            kind: role.into(),
            sha256: sha,
            size_bytes: artifact.size_bytes,
        });
        staged.push(artifact);
    }
    Ok((
        AudioAssemblyPublication {
            assembly_id: intent.assembly_id,
            attempt_id: attempt,
            sample_rate: 48000,
            channels: 1,
            frames: 4800,
            pcm_sha256: Sha256Digest::parse(source::digest(&wave[44..]))?,
            peak_sample_abs: 200,
            full_scale_samples: 0,
            files,
            ready: false,
        },
        staged,
    ))
}

async fn finalize(
    fixture: &AssemblySource,
    artifact: &StagedArtifact,
) -> Result<(), Box<dyn std::error::Error>> {
    fixture
        .files
        .finalize_staged_artifact(
            &artifact.staging_key,
            &artifact.final_key,
            Some(artifact.size_bytes),
        )
        .await?;
    sqlx::query("UPDATE artifacts SET state='ready',ready_at=? WHERE id=?")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(artifact.artifact.id.to_string())
        .execute(&fixture.pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn immutable_registration_is_idempotent_and_checks_exact_ready_source()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let store = SqliteAudioAssemblyStore::new(fixture.pool.clone());
    let first = store.register(&fixture.intent()?).await?;
    let duplicate = store.register(&fixture.intent()?).await?;
    assert_eq!(first, duplicate);
    let mut plan = fixture.plan()?;
    plan.timeline.utterances[0].pause_after_ms = 10;
    let mut collision = fixture.with_plan(plan)?;
    collision.assembly_id = first.assembly_id;
    assert!(store.register(&collision).await.is_err());
    for variant in 0..4 {
        let mut plan = fixture.plan()?;
        match variant {
            0 => plan.project_id = domain::project::ProjectId::new(),
            1 => plan.raw_wav.sha256 = Sha256Digest::parse("0".repeat(64))?,
            2 => plan.timeline.source_plan_sha256 = Sha256Digest::parse("0".repeat(64))?,
            _ => {
                plan.timeline.expected_utterance_ids[0] = domain::speech::SpeechId::parse("other")?;
                plan.timeline.utterances[0].utterance_id =
                    domain::speech::SpeechId::parse("other")?;
            }
        }
        assert!(store.register(&fixture.with_plan(plan)?).await.is_err());
    }
    assert_eq!(store.list_ids(&fixture.project_id).await?.len(), 1);
    assert_eq!(
        fixture
            .index
            .list_by_project(&fixture.project_id)
            .await?
            .len(),
        4
    );
    fixture.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn competing_attempts_and_stale_revisions_cannot_advance_or_complete()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let store = SqliteAudioAssemblyStore::new(fixture.pool.clone());
    let (intent, attempt, mut job) = running(&fixture, &store).await?;
    assert!(
        store
            .create_pending(
                intent.assembly_id,
                Uuid::new_v4(),
                Job::new(
                    fixture.project_id.clone(),
                    "Competing".into(),
                    JobKind::AudioAssembly
                )
            )
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM jobs WHERE kind='audioAssembly'")
            .fetch_one(&fixture.pool)
            .await?,
        1
    );
    let revision = job.revision();
    job.advance(
        domain::dubbing::DubbingPipelineStage::PostprocessAudio,
        domain::job::JobProgress {
            percent: 35,
            message: "Assembly".into(),
            current_step: None,
            processed_items: None,
            total_items: None,
        },
    )?;
    assert!(store.update(&job, Uuid::new_v4(), revision).await.is_err());
    store.update(&job, attempt, revision).await?;
    assert!(store.update(&job, attempt, revision).await.is_err());
    let mut premature = job.clone();
    premature.mark_completed()?;
    assert!(
        store
            .update(&premature, attempt, job.revision())
            .await
            .is_err()
    );
    let revision = job.revision();
    job.cancel()?;
    store.update(&job, attempt, revision).await?;
    assert!(store.active(intent.assembly_id).await?.is_none());
    store
        .create_pending(
            intent.assembly_id,
            Uuid::new_v4(),
            Job::new(
                fixture.project_id.clone(),
                "Resume".into(),
                JobKind::AudioAssembly,
            ),
        )
        .await?;
    fixture.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn publication_stays_hidden_until_every_output_is_ready_and_recovers_after_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let store = SqliteAudioAssemblyStore::new(fixture.pool.clone());
    let (intent, attempt, job) = running(&fixture, &store).await?;
    let (publication, staged) = output(&fixture, &intent, attempt).await?;
    store.publish(&job, &publication, &staged).await?;
    assert!(store.publish(&job, &publication, &staged).await.is_err());
    finalize(&fixture, &staged[0]).await?;
    assert!(
        fixture
            .index
            .get(&publication.files[0].artifact_id)
            .await?
            .is_none()
    );
    assert!(
        store
            .complete_publication(intent.assembly_id)
            .await?
            .is_none()
    );
    finalize(&fixture, &staged[1]).await?;
    fixture.pool.close().await;
    let reopened = connect_sqlite(&fixture.db_path).await?;
    let store = SqliteAudioAssemblyStore::new(reopened.clone());
    let index = SqliteArtifactIndex::new(reopened.clone());
    assert!(
        index
            .get(&publication.files[0].artifact_id)
            .await?
            .is_none()
    );
    let completed = store
        .complete_publication(intent.assembly_id)
        .await?
        .ok_or("Assembly did not reconcile")?;
    assert_eq!(completed.status(), &JobStatus::Completed);
    assert!(
        store
            .publication(intent.assembly_id)
            .await?
            .ok_or("Missing assembly publication")?
            .ready
    );
    assert!(
        index
            .get(&publication.files[0].artifact_id)
            .await?
            .is_some()
    );
    assert_eq!(index.list_by_project(&fixture.project_id).await?.len(), 6);
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn discarded_partial_output_is_tombstoned_and_late_finalize_cannot_resurrect_it()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let store = SqliteAudioAssemblyStore::new(fixture.pool.clone());
    let (intent, attempt, mut job) = running(&fixture, &store).await?;
    let (publication, staged) = output(&fixture, &intent, attempt).await?;
    store.publish(&job, &publication, &staged).await?;
    finalize(&fixture, &staged[0]).await?;
    let message:String = sqlx::query_scalar("SELECT id FROM outbox_messages WHERE json_extract(payload_json,'$.artifact_id')=? AND json_extract(payload_json,'$.type')='finalize_staged_artifact'").bind(publication.files[0].artifact_id.to_string()).fetch_one(&fixture.pool).await?;
    sqlx::query("UPDATE outbox_messages SET status='processing',locked_by='late-claim' WHERE id=?")
        .bind(&message)
        .execute(&fixture.pool)
        .await?;
    assert!(store.discard_unready(intent.assembly_id, attempt).await?);
    assert!(!store.discard_unready(intent.assembly_id, attempt).await?);
    assert!(matches!(
        fixture
            .index
            .get_for_finalization(&fixture.project_id, &publication.files[0].artifact_id)
            .await?,
        FinalizationMetadata::ArtifactDiscarded
    ));
    let result = SqliteStorageUnitOfWork::new(fixture.pool.clone())
        .commit_artifact_finalize(CommitArtifactFinalize {
            message_id: OutboxMessageId(Uuid::parse_str(&message)?),
            claim_token: Some("late-claim".into()),
            project_id: fixture.project_id.clone(),
            artifact_id: publication.files[0].artifact_id.clone(),
            ready_key: staged[0].final_key.clone(),
        })
        .await?;
    assert_eq!(
        result,
        CommitArtifactFinalizeResult::ObsoleteBecauseArtifactDiscarded
    );
    assert_eq!(
        fixture
            .index
            .list_by_project(&fixture.project_id)
            .await?
            .len(),
        4
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audio_assembly_discarded_artifacts")
            .fetch_one(&fixture.pool)
            .await?,
        2
    );
    let revision = job.revision();
    job.cancel()?;
    store.update(&job, attempt, revision).await?;
    store
        .create_pending(
            intent.assembly_id,
            Uuid::new_v4(),
            Job::new(
                fixture.project_id.clone(),
                "Resume".into(),
                JobKind::AudioAssembly,
            ),
        )
        .await?;
    assert!(store.publish(&job, &publication, &staged).await.is_err());
    fixture.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn impossible_timing_and_output_reuse_of_a_raw_artifact_are_refused()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let store = SqliteAudioAssemblyStore::new(fixture.pool.clone());
    let mut plan = fixture.plan()?;
    plan.timeline.mode = domain::speech::TimingMode::SubtitleWindow;
    plan.timeline.source_duration_ms = Some(500);
    plan.timeline.utterances[0].source_window = Some(domain::speech::timeline::SourceWindow {
        start_ms: 100,
        end_ms: 101,
    });
    assert!(store.register(&fixture.with_plan(plan)?).await.is_err());
    let (intent, attempt, job) = running(&fixture, &store).await?;
    let (mut publication, mut staged) = output(&fixture, &intent, attempt).await?;
    publication.files[0].artifact_id = fixture.reference("wav")?.artifact_id;
    staged[0].artifact.id = publication.files[0].artifact_id.clone();
    assert!(store.publish(&job, &publication, &staged).await.is_err());
    assert!(store.publication(intent.assembly_id).await?.is_none());
    assert_eq!(
        fixture
            .index
            .list_by_project(&fixture.project_id)
            .await?
            .len(),
        4
    );
    fixture.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn schema18_upgrade_keeps_ready_raw_speech_and_foreign_keys()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let before = fixture.source.clone();
    let ids: Vec<_> = fixture
        .publication
        .files
        .iter()
        .map(|file| file.artifact_id.clone())
        .collect();
    fixture.pool.close().await;
    let raw = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&fixture.db_path)
                .foreign_keys(true),
        )
        .await?;
    for statement in [
        "DROP TABLE audio_assembly_artifacts",
        "DROP TABLE audio_assembly_publications",
        "DROP TABLE audio_assembly_discarded_artifacts",
        "DROP TABLE audio_assembly_attempts",
        "DROP TABLE audio_assembly_intents",
    ] {
        sqlx::query(statement).execute(&raw).await?;
    }
    sqlx::query("PRAGMA user_version=18").execute(&raw).await?;
    raw.close().await;
    let reopened = connect_sqlite(&fixture.db_path).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA user_version")
            .fetch_one(&reopened)
            .await?,
        20
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&reopened)
            .await?
            .is_empty()
    );
    let speech = SqliteSpeechRunStore::new(reopened.clone());
    use ports::speech_run::SpeechRunStore;
    assert_eq!(speech.get_run(before.run_id).await?, Some(before));
    assert!(
        speech
            .publication(fixture.source.run_id)
            .await?
            .ok_or("Raw publication missing after upgrade")?
            .ready
    );
    let index = SqliteArtifactIndex::new(reopened.clone());
    for id in ids {
        assert!(index.get(&id).await?.is_some());
    }
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn derived_history_protects_raw_source_and_project_delete_cascades_without_orphans()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let store = SqliteAudioAssemblyStore::new(fixture.pool.clone());
    let (intent, attempt, job) = running(&fixture, &store).await?;
    let (publication, staged) = output(&fixture, &intent, attempt).await?;
    store.publish(&job, &publication, &staged).await?;
    assert!(
        sqlx::query("DELETE FROM speech_run_intents WHERE run_id=?")
            .bind(fixture.source.run_id.to_string())
            .execute(&fixture.pool)
            .await
            .is_err()
    );
    assert!(store.get(intent.assembly_id).await?.is_some());
    sqlx::query("DELETE FROM projects WHERE id=?")
        .bind(fixture.project_id.to_string())
        .execute(&fixture.pool)
        .await?;
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&fixture.pool)
            .await?
            .is_empty()
    );
    assert!(store.get(intent.assembly_id).await?.is_none());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audio_assembly_artifacts")
            .fetch_one(&fixture.pool)
            .await?,
        0
    );
    fixture.pool.close().await;
    Ok(())
}
