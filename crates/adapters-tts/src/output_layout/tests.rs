use super::*;
use domain::{
    project::ProjectId,
    speech::{Sha256Digest, SpeechId},
};
use ports::speech_audio::UtteranceAudioRange;

fn layout(path: &Path) -> Result<SpeechAudioLayout, Box<dyn std::error::Error>> {
    let hash = Sha256Digest::parse("a".repeat(64))?;
    Ok(SpeechAudioLayout {
        aggregate_path: path.join("raw.wav"),
        aggregate_sha256: hash.clone(),
        source_plan_sha256: hash.clone(),
        raw_manifest_sha256: hash,
        assembly_policy: AUDIO_LAYOUT_POLICY.into(),
        sample_rate: 48000,
        channels: 1,
        frame_count: 120,
        utterances: vec![UtteranceAudioRange {
            utterance_id: SpeechId::parse("one")?,
            first_frame: 0,
            frame_count: 120,
            chunk_ids: vec!["one-chunk-0001".into()],
            source_window: None,
        }],
    })
}

fn request(path: &Path) -> (TtsExecutionRequest, TtsCompleted) {
    let output = path.join("raw.wav");
    let manifest = path.join("manifest.json");
    (
        TtsExecutionRequest {
            project_id: ProjectId::new(),
            run_id: "run".into(),
            attempt_id: "attempt".into(),
            plan_digest: "a".repeat(64),
            expected_utterances: 1,
            expected_chunks: 1,
            plan_path: path.join("plan.json"),
            profile_path: path.join("profile.json"),
            out_file: output.clone(),
            manifest_path: manifest.clone(),
            cache_dir: path.join("cache"),
            checkpoint_root: path.join("checkpoints"),
            rebuild: false,
        },
        TtsCompleted {
            output,
            output_sha256: "a".repeat(64),
            manifest,
            manifest_sha256: "b".repeat(64),
            sample_rate: 48000,
            channels: 1,
            frames: 120,
            duration_ms: 2,
            utterance_count: 1,
            chunk_count: 1,
        },
    )
}

#[test]
fn immutable_layout_file_hashes_the_exact_serialized_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = layout(root.path())?;
    let path = root.path().join("audio-layout.json");
    let file = write_layout(&source, &path, &CancellationToken::new())?;
    let bytes = std::fs::read(path)?;
    assert_eq!(file.kind, "audio_layout");
    assert_eq!(file.sha256, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(serde_json::from_slice::<SpeechAudioLayout>(&bytes)?, source);
    assert_eq!(std::fs::read_dir(root.path())?.count(), 1);
    Ok(())
}

#[test]
fn earlier_layout_is_preserved_and_temporary_file_is_cleaned_on_collision()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("audio-layout.json");
    std::fs::write(&path, b"accepted earlier layout")?;
    assert!(write_layout(&layout(root.path())?, &path, &CancellationToken::new()).is_err());
    assert_eq!(std::fs::read(path)?, b"accepted earlier layout");
    assert_eq!(std::fs::read_dir(root.path())?.count(), 1);
    Ok(())
}

#[test]
fn cancelled_layout_creation_writes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        write_layout(
            &layout(root.path())?,
            &root.path().join("audio-layout.json"),
            &cancel
        ),
        Err(PortError::Cancelled)
    ));
    assert_eq!(std::fs::read_dir(root.path())?.count(), 0);
    Ok(())
}

#[tokio::test]
async fn unknown_policy_keeps_legacy_output_without_inventing_a_layout()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let (request, completed) = request(root.path());
    assert!(
        produce(
            request,
            completed,
            &serde_json::json!({"audio_assembly":"legacy"}),
            CancellationToken::new()
        )
        .await?
        .is_none()
    );
    assert_eq!(std::fs::read_dir(root.path())?.count(), 0);
    Ok(())
}

#[tokio::test]
async fn known_policy_requires_successful_result_verification()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let (request, completed) = request(root.path());
    assert!(
        produce(
            request,
            completed,
            &serde_json::json!({"audio_assembly":AUDIO_LAYOUT_POLICY}),
            CancellationToken::new()
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read_dir(root.path())?.count(), 0);
    Ok(())
}

#[test]
fn verification_scope_drop_cancels_only_its_child_token() {
    let parent = CancellationToken::new();
    let child = parent.child_token();
    let guard = VerificationGuard(child.clone());
    drop(guard);
    assert!(child.is_cancelled());
    assert!(!parent.is_cancelled());
}
