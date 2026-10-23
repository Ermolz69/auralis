use super::*;
use domain::speech::{TimingMode, timeline::AudioTimelineRequest};

#[tokio::test]
async fn cancelled_request_never_needs_an_installed_tool_or_creates_staging() {
    let parent =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/revoice/assembly-unit");
    assert!(fs::create_dir_all(&parent).is_ok());
    let directory = match tempfile::tempdir_in(parent) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let pin = match Sha256Digest::parse("0".repeat(64)) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let assembler = match FfmpegAudioAssembler::new(
        directory.path().join("missing.exe"),
        pin.clone(),
        directory.path().into(),
        directory.path().into(),
        AudioAssemblyLimits::default(),
    ) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let cancel = CancellationToken::new();
    cancel.cancel();
    let request = AudioAssemblyRequest {
        timeline: AudioTimelineRequest {
            source_plan_sha256: pin.clone(),
            raw_manifest_sha256: pin,
            mode: TimingMode::Continuous,
            expected_utterance_ids: vec![],
            utterances: vec![],
            source_duration_ms: None,
        },
        inputs: vec![],
    };
    assert!(matches!(
        assembler.assemble(request, cancel).await,
        Err(PortError::Cancelled)
    ));
    assert_eq!(
        fs::read_dir(directory.path())
            .map(|items| items.count())
            .ok(),
        Some(0)
    );
}

#[test]
fn unsafe_limits_are_refused_before_work() {
    let mut limits = AudioAssemblyLimits {
        max_input_bytes: 0,
        ..AudioAssemblyLimits::default()
    };
    assert!(limits.validate().is_err());
    limits = AudioAssemblyLimits {
        max_stderr_bytes: 65537,
        ..AudioAssemblyLimits::default()
    };
    assert!(limits.validate().is_err());
    limits = AudioAssemblyLimits {
        total_timeout: std::time::Duration::from_secs(1),
        ..AudioAssemblyLimits::default()
    };
    assert!(limits.validate().is_err());
}
