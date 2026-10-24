#[path = "support/audio_assembly_fixture.rs"]
mod fixture;
#[path = "support/observed_runtime.rs"]
mod observed_runtime;
#[allow(dead_code)]
#[path = "../../adapters-storage/tests/support/assembly_source.rs"]
mod source;

use application::usecases::audio_assembly::PrepareAssemblyRequest;
use domain::{
    job::{Job, JobKind, JobStatus},
    media::ArtifactKind,
    project::ProjectId,
    speech::TimingMode,
};
use fixture::{Fixture, TestResult};
use ports::{
    artifact_index::ArtifactIndex, cancellation::CancellationToken,
    job_runtime_control::JobRuntimeControlPort, job_scheduler::JobSchedulerPort,
    storage::ArtifactStore,
};
use std::{sync::atomic::Ordering, time::Duration};
use uuid::Uuid;

#[tokio::test]
async fn exact_review_rejects_stale_settings_and_explicit_timing_conflicts() -> TestResult {
    let fixture = Fixture::new().await?;
    let preview = fixture
        .cases
        .inspect(
            fixture.source.project_id.clone(),
            fixture.request(0),
            CancellationToken::new(),
        )
        .await?;
    assert_eq!(preview.frames, Some(4800));
    assert!(preview.conflicts.is_empty());
    assert_eq!(
        preview.utterances[0].audio_sha256,
        fixture.source.reference("wav")?.sha256
    );
    for (pause, reviewed) in [(0, false), (10, true)] {
        assert!(
            fixture
                .cases
                .prepare(
                    fixture.source.project_id.clone(),
                    PrepareAssemblyRequest {
                        source: fixture.request(pause),
                        expected_preview_sha256: preview.preview_sha256.clone(),
                        reviewed
                    }
                )
                .await
                .is_err()
        );
    }
    let first = fixture.prepare(0).await?;
    let again = fixture.prepare(0).await?;
    assert_eq!(first.assembly_id, again.assembly_id);
    assert!(
        fixture
            .cases
            .state(ProjectId::new(), first.assembly_id)
            .await
            .is_err()
    );
    let mut request = fixture.request(0);
    request.settings.mode = TimingMode::SubtitleWindow;
    request.settings.source_duration_ms = Some(50);
    let conflict = fixture
        .cases
        .inspect(
            fixture.source.project_id.clone(),
            request.clone(),
            CancellationToken::new(),
        )
        .await?;
    assert!(conflict.frames.is_none());
    assert!(!conflict.conflicts.is_empty());
    assert!(
        fixture
            .cases
            .prepare(
                fixture.source.project_id.clone(),
                PrepareAssemblyRequest {
                    source: request,
                    expected_preview_sha256: conflict.preview_sha256,
                    reviewed: true
                }
            )
            .await
            .is_err()
    );
    assert_eq!(fixture.renderer.calls.load(Ordering::SeqCst), 0);
    fixture.close().await
}

#[tokio::test]
async fn actual_outbox_publishes_distinct_wav_and_retains_raw_source_and_project() -> TestResult {
    let fixture = Fixture::new().await?;
    let before = fixture
        .cases
        .projects
        .get(&fixture.source.project_id)
        .await?;
    let raw = fixture.source.reference("wav")?;
    let prepared = fixture.prepare(25).await?;
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    let publication = fixture.publication(prepared.assembly_id).await?;
    assert!(!publication.ready);
    for file in &publication.files {
        assert!(fixture.source.index.get(&file.artifact_id).await?.is_none());
    }
    let completed = fixture.completed(prepared.assembly_id).await?;
    let job = completed.job.ok_or("Missing completed job")?;
    assert_eq!(job.kind, JobKind::AudioAssembly);
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.progress.percent, 100);
    let result = completed.result.ok_or("Missing result")?;
    assert_eq!(result.frames, 6000);
    let wav = result
        .files
        .iter()
        .find(|file| file.kind == "wav")
        .ok_or("Missing WAV")?;
    assert_ne!(wav.artifact_id, raw.artifact_id);
    let artifact = fixture
        .source
        .index
        .get(&wav.artifact_id)
        .await?
        .ok_or("WAV hidden after ready")?;
    assert_eq!(artifact.kind, ArtifactKind::MixedAudio);
    let bytes = std::fs::read(fixture.source.files.resolve_artifact(&artifact).await?)?;
    assert_eq!(&bytes[44..9644], &source::wave()[44..]);
    assert!(bytes[9644..].iter().all(|byte| *byte == 0));
    let original = fixture
        .source
        .index
        .get(&raw.artifact_id)
        .await?
        .ok_or("Raw WAV missing")?;
    assert_eq!(
        source::digest(&std::fs::read(
            fixture.source.files.resolve_artifact(&original).await?
        )?),
        raw.sha256.as_str()
    );
    assert_eq!(
        fixture
            .cases
            .projects
            .get(&fixture.source.project_id)
            .await?,
        before
    );
    for resume in [true, false] {
        assert!(
            fixture
                .cases
                .start(
                    fixture.source.project_id.clone(),
                    prepared.assembly_id,
                    resume
                )
                .await?
                .result
                .is_some()
        );
    }
    assert_eq!(fixture.renderer.calls.load(Ordering::SeqCst), 1);
    assert_eq!(std::fs::read_dir(&fixture.renderer.root)?.count(), 0);
    assert_eq!(
        fixture
            .cases
            .list(fixture.source.project_id.clone())
            .await?
            .len(),
        1
    );
    fixture.close().await
}

#[tokio::test]
async fn common_cancel_during_render_and_resume_preserve_the_approved_source() -> TestResult {
    let fixture = Fixture::new().await?;
    fixture.renderer.hold.store(true, Ordering::SeqCst);
    let before = fixture
        .cases
        .projects
        .get(&fixture.source.project_id)
        .await?;
    let prepared = fixture.prepare(0).await?;
    let started = fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    tokio::time::timeout(Duration::from_secs(15), fixture.renderer.entered.notified()).await?;
    assert!(
        fixture
            .cases
            .start(
                fixture.source.project_id.clone(),
                prepared.assembly_id,
                false
            )
            .await
            .is_err()
    );
    let cancelled = fixture
        .manager
        .cancel_job(&started.job.ok_or("Missing job")?.id)
        .await?;
    assert_eq!(cancelled.status, JobStatus::Cancelled);
    let state = fixture
        .cases
        .state(fixture.source.project_id.clone(), prepared.assembly_id)
        .await?;
    assert!(state.result.is_none());
    assert!(
        fixture
            .cases
            .store
            .active(prepared.assembly_id)
            .await?
            .is_none()
    );
    assert_eq!(
        fixture
            .cases
            .projects
            .get(&fixture.source.project_id)
            .await?,
        before
    );
    fixture.renderer.hold.store(false, Ordering::SeqCst);
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            true,
        )
        .await?;
    assert!(
        fixture
            .completed(prepared.assembly_id)
            .await?
            .result
            .is_some()
    );
    assert_eq!(fixture.renderer.calls.load(Ordering::SeqCst), 2);
    fixture.close().await
}

#[tokio::test]
async fn cancel_pending_publication_discards_late_outbox_files_and_allows_resume() -> TestResult {
    let fixture = Fixture::new().await?;
    let prepared = fixture.prepare(0).await?;
    let started = fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    let partial = fixture.publication(prepared.assembly_id).await?;
    fixture
        .manager
        .cancel_job(&started.job.ok_or("Missing job")?.id)
        .await?;
    fixture
        .cases
        .state(fixture.source.project_id.clone(), prepared.assembly_id)
        .await?;
    fixture.flush().await?;
    assert!(
        fixture
            .cases
            .store
            .publication(prepared.assembly_id)
            .await?
            .is_none()
    );
    for file in partial.files {
        assert!(fixture.source.index.get(&file.artifact_id).await?.is_none());
    }
    assert_eq!(
        fixture
            .source
            .index
            .list_by_project(&fixture.source.project_id)
            .await?
            .len(),
        4
    );
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            true,
        )
        .await?;
    assert!(
        fixture
            .completed(prepared.assembly_id)
            .await?
            .result
            .is_some()
    );
    fixture.close().await
}

#[tokio::test]
async fn pending_publication_recovers_without_a_second_render_after_actor_abort() -> TestResult {
    let fixture = Fixture::new().await?;
    let prepared = fixture.prepare(0).await?;
    let started = fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    fixture.publication(prepared.assembly_id).await?;
    fixture
        .manager
        .rollback_runtime_start(&started.job.ok_or("Missing job")?.id)
        .await?;
    fixture.cases.recover().await?;
    assert!(
        fixture
            .cases
            .state(fixture.source.project_id.clone(), prepared.assembly_id)
            .await?
            .result
            .is_none()
    );
    let mut recovered = fixture.cases.clone();
    recovered.renderer = None;
    fixture.flush().await?;
    recovered.recover().await?;
    let result = recovered
        .state(fixture.source.project_id.clone(), prepared.assembly_id)
        .await?;
    assert!(result.result.is_some());
    assert_eq!(
        result.job.ok_or("Missing job")?.status,
        JobStatus::Completed
    );
    assert_eq!(fixture.renderer.calls.load(Ordering::SeqCst), 1);
    fixture.close().await
}

#[tokio::test]
async fn orphan_attempt_requires_explicit_resume_and_missing_renderer_is_actionable() -> TestResult
{
    let fixture = Fixture::new().await?;
    let prepared = fixture.prepare(0).await?;
    let mut absent = fixture.cases.clone();
    absent.renderer = None;
    absent.renderer_error = Some("Repair managed FFmpeg fixture".into());
    let preview = absent
        .inspect(
            fixture.source.project_id.clone(),
            fixture.request(0),
            CancellationToken::new(),
        )
        .await?;
    assert!(!preview.renderer_available);
    assert!(
        absent
            .start(
                fixture.source.project_id.clone(),
                prepared.assembly_id,
                false
            )
            .await
            .is_err()
    );
    fixture
        .cases
        .store
        .create_pending(
            prepared.assembly_id,
            Uuid::new_v4(),
            Job::new(
                fixture.source.project_id.clone(),
                "Orphan fixture".into(),
                JobKind::AudioAssembly,
            ),
        )
        .await?;
    assert!(
        fixture
            .cases
            .start(
                fixture.source.project_id.clone(),
                prepared.assembly_id,
                true
            )
            .await
            .is_err()
    );
    fixture.cases.recover().await?;
    let state = fixture
        .cases
        .state(fixture.source.project_id.clone(), prepared.assembly_id)
        .await?;
    assert_eq!(
        state.job.ok_or("Missing recovered job")?.status,
        JobStatus::Failed
    );
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            true,
        )
        .await?;
    assert!(
        fixture
            .completed(prepared.assembly_id)
            .await?
            .result
            .is_some()
    );
    fixture.close().await
}

#[tokio::test]
async fn corruption_after_review_is_rejected_before_the_renderer_starts() -> TestResult {
    let fixture = Fixture::new().await?;
    let prepared = fixture.prepare(0).await?;
    let raw = fixture.source.reference("wav")?;
    let artifact = fixture
        .source
        .index
        .get(&raw.artifact_id)
        .await?
        .ok_or("Raw missing")?;
    let path = fixture.source.files.resolve_artifact(&artifact).await?;
    let mut changed = std::fs::read(&path)?;
    changed[100] ^= 1;
    std::fs::write(path, changed)?;
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let state = fixture
                .cases
                .state(fixture.source.project_id.clone(), prepared.assembly_id)
                .await?;
            if state.job.is_some_and(|job| job.status == JobStatus::Failed) {
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    assert_eq!(fixture.renderer.calls.load(Ordering::SeqCst), 0);
    assert!(
        fixture
            .cases
            .store
            .publication(prepared.assembly_id)
            .await?
            .is_none()
    );
    fixture.close().await
}

#[tokio::test]
async fn playback_detects_later_output_corruption_while_history_remains_metadata_only() -> TestResult
{
    let fixture = Fixture::new().await?;
    let prepared = fixture.prepare(0).await?;
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    let complete = fixture.completed(prepared.assembly_id).await?;
    assert!(
        fixture
            .cases
            .verify(fixture.source.project_id.clone(), prepared.assembly_id)
            .await?
            .result
            .is_some()
    );
    let wav = complete
        .result
        .ok_or("Missing result")?
        .files
        .into_iter()
        .find(|file| file.kind == "wav")
        .ok_or("Missing WAV")?;
    let artifact = fixture
        .source
        .index
        .get(&wav.artifact_id)
        .await?
        .ok_or("Missing artifact")?;
    let path = fixture.source.files.resolve_artifact(&artifact).await?;
    let mut bytes = std::fs::read(&path)?;
    bytes[100] ^= 1;
    std::fs::write(path, bytes)?;
    assert_eq!(
        fixture
            .cases
            .list(fixture.source.project_id.clone())
            .await?
            .len(),
        1
    );
    assert!(
        fixture
            .cases
            .verify(fixture.source.project_id.clone(), prepared.assembly_id)
            .await
            .is_err()
    );
    fixture.close().await
}

#[tokio::test]
async fn renderer_panic_leaves_a_recoverable_attempt_with_no_ready_output() -> TestResult {
    let fixture = Fixture::new().await?;
    fixture.renderer.panic_once.store(true, Ordering::SeqCst);
    let prepared = fixture.prepare(0).await?;
    let started = fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    tokio::time::timeout(Duration::from_secs(15), fixture.renderer.entered.notified()).await?;
    fixture
        .manager
        .cancel_and_evict_jobs(&[started.job.ok_or("Missing job")?.id])
        .await?;
    fixture.cases.recover().await?;
    let state = fixture
        .cases
        .state(fixture.source.project_id.clone(), prepared.assembly_id)
        .await?;
    assert_eq!(
        state.job.ok_or("Missing failed job")?.status,
        JobStatus::Failed
    );
    assert!(state.result.is_none());
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            true,
        )
        .await?;
    assert!(
        fixture
            .completed(prepared.assembly_id)
            .await?
            .result
            .is_some()
    );
    fixture.close().await
}

#[tokio::test]
async fn source_window_publication_keeps_complete_speech_at_its_original_start() -> TestResult {
    let fixture = Fixture::new().await?;
    let mut request = fixture.request(0);
    request.settings.mode = TimingMode::SubtitleWindow;
    request.settings.source_duration_ms = Some(600);
    let preview = fixture
        .cases
        .inspect(
            fixture.source.project_id.clone(),
            request.clone(),
            CancellationToken::new(),
        )
        .await?;
    assert_eq!(preview.frames, Some(28800));
    assert!(preview.conflicts.is_empty());
    let prepared = fixture
        .cases
        .prepare(
            fixture.source.project_id.clone(),
            PrepareAssemblyRequest {
                source: request,
                expected_preview_sha256: preview.preview_sha256,
                reviewed: true,
            },
        )
        .await?;
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    let complete = fixture.completed(prepared.assembly_id).await?;
    let file = complete
        .result
        .ok_or("Missing timed result")?
        .files
        .into_iter()
        .find(|file| file.kind == "wav")
        .ok_or("Missing timed WAV")?;
    let artifact = fixture
        .source
        .index
        .get(&file.artifact_id)
        .await?
        .ok_or("Timed WAV is not ready")?;
    let bytes = std::fs::read(fixture.source.files.resolve_artifact(&artifact).await?)?;
    assert_eq!(bytes.len(), 57644);
    assert!(bytes[44..9644].iter().all(|byte| *byte == 0));
    assert_eq!(&bytes[9644..19244], &source::wave()[44..]);
    assert!(bytes[19244..].iter().all(|byte| *byte == 0));
    fixture.close().await
}

#[tokio::test]
async fn legacy_raw_result_without_a_layout_requires_an_explicit_new_render() -> TestResult {
    let fixture = Fixture::new().await?;
    let mut legacy = fixture.source.publication.clone();
    legacy.files.retain(|file| file.kind != "audio_layout");
    sqlx::query("UPDATE speech_publications SET publication_json=? WHERE run_id=?")
        .bind(serde_json::to_string(&legacy)?)
        .bind(fixture.source.source.run_id.to_string())
        .execute(&fixture.source.pool)
        .await?;
    let error = fixture
        .cases
        .inspect(
            fixture.source.project_id.clone(),
            fixture.request(0),
            CancellationToken::new(),
        )
        .await
        .err()
        .ok_or("Legacy raw result must require a new layout")?;
    assert!(
        error
            .to_string()
            .contains("install the streaming TTS package and render a new result")
    );
    assert_eq!(fixture.renderer.calls.load(Ordering::SeqCst), 0);
    assert!(
        fixture
            .cases
            .list(fixture.source.project_id.clone())
            .await?
            .is_empty()
    );
    assert_eq!(
        fixture
            .source
            .index
            .list_by_project(&fixture.source.project_id)
            .await?
            .len(),
        4
    );
    fixture.close().await
}

#[tokio::test]
async fn unconfirmed_child_cleanup_remains_registered_and_cannot_be_reported_cancelled()
-> TestResult {
    let mut fixture = Fixture::new().await?;
    let observed = std::sync::Arc::new(observed_runtime::ObservedRuntime::new(
        fixture.manager.clone(),
    ));
    fixture.cases.runtime = observed.clone();
    fixture
        .renderer
        .cleanup_uncertain
        .store(true, Ordering::SeqCst);
    let prepared = fixture.prepare(0).await?;
    let started = fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(15), observed.completion()?.wait()).await?,
        ports::job_runtime_control::RuntimeTaskOutcome::CleanupUnconfirmed
    );
    assert!(
        fixture
            .manager
            .cancel_job(&started.job.ok_or("Missing job")?.id)
            .await
            .is_err()
    );
    let state = fixture
        .cases
        .state(fixture.source.project_id.clone(), prepared.assembly_id)
        .await?;
    assert_eq!(
        state.job.ok_or("Missing retained job")?.status,
        JobStatus::Cancelling
    );
    assert!(state.result.is_none());
    assert!(
        fixture
            .cases
            .start(
                fixture.source.project_id.clone(),
                prepared.assembly_id,
                true
            )
            .await
            .is_err()
    );
    let report = fixture.manager.drain_all(Duration::from_secs(5)).await?;
    assert_eq!(report.unconfirmed_count, 1);
    fixture.source.pool.close().await;
    Ok(())
}
