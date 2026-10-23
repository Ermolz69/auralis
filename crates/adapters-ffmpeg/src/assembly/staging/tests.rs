use super::*;
use crate::assembly::control::WorkControl;
use domain::speech::{
    Sha256Digest, SpeechId, TimingMode,
    timeline::{AudioTimelineRequest, MeasuredUtterance, SourceWindow, plan_audio_timeline},
};
use ports::{audio_assembly::UtteranceAudioFile, cancellation::CancellationToken};
use std::time::Duration;

fn id(value: &str) -> SpeechId {
    match SpeechId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

fn sha(value: &str) -> Sha256Digest {
    match Sha256Digest::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

fn control() -> WorkControl {
    match WorkControl::new(CancellationToken::new(), Duration::from_secs(10)) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

fn fixture() -> (TempDir, AudioAssemblyRequest) {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/revoice/assembly-unit");
    assert!(fs::create_dir_all(&parent).is_ok());
    let directory = match tempfile::tempdir_in(parent) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    assert!(fs::create_dir(directory.path().join("inputs")).is_ok());
    assert!(fs::create_dir(directory.path().join("staging")).is_ok());
    let path = directory.path().join("inputs/Кит'а中文.wav");
    let samples = [1_i16, -2, i16::MIN, i16::MAX];
    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&44_u32.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&48000_u32.to_le_bytes());
    wav.extend_from_slice(&96000_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&8_u32.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    assert!(fs::write(&path, &wav).is_ok());
    let digest = sha(&format!("{:x}", Sha256::digest(&wav)));
    let facts = MeasuredUtterance {
        utterance_id: id("one"),
        audio_sha256: digest.clone(),
        sample_rate: 48000,
        channels: 1,
        frame_count: 4,
        source_window: Some(SourceWindow {
            start_ms: 1,
            end_ms: 2,
        }),
        pause_after_ms: 0,
    };
    let timeline = AudioTimelineRequest {
        source_plan_sha256: digest.clone(),
        raw_manifest_sha256: digest,
        mode: TimingMode::SubtitleWindow,
        expected_utterance_ids: vec![id("one")],
        utterances: vec![facts],
        source_duration_ms: Some(3),
    };
    (
        directory,
        AudioAssemblyRequest {
            timeline,
            inputs: vec![UtteranceAudioFile {
                utterance_id: id("one"),
                path,
            }],
        },
    )
}

fn prepare_case(
    directory: &TempDir,
    input: &AudioAssemblyRequest,
    limits: &AudioAssemblyLimits,
) -> Result<PreparedAssembly, PortError> {
    let timeline = match plan_audio_timeline(&input.timeline) {
        Ok(value) => value,
        Err(error) => panic!("{error:?}"),
    };
    prepare(
        input,
        timeline,
        &fs::canonicalize(directory.path().join("inputs")).map_err(error::io)?,
        &directory.path().join("staging"),
        limits,
        &control(),
    )
}

#[test]
fn owned_snapshot_preserves_pcm_and_uses_only_generated_safe_names() {
    let (directory, input) = fixture();
    let original = match fs::read(&input.inputs[0].path) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let prepared = match prepare_case(&directory, &input, &AudioAssemblyLimits::default()) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let mut expected = Sha256::new();
    expected.update([0_u8; 96]);
    expected.update(&original[44..]);
    expected.update([0_u8; 184]);
    assert_eq!(prepared.pcm_sha256, format!("{:x}", expected.finalize()));
    assert_eq!(
        prepared.stats,
        PcmStats {
            peak: 32768,
            full_scale: 2,
            samples: 144
        }
    );
    let script = match fs::read_to_string(prepared.directory.path().join("timeline.ffconcat")) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(
        script,
        "ffconcat version 1.0\nfile 'silence-00000.wav'\nfile 'speech-00000.wav'\nfile 'silence-00002.wav'\n"
    );
    assert!(!script.contains("Кит"));
    drop(prepared);
    assert!(matches!(fs::read(&input.inputs[0].path), Ok(value) if value == original));
    assert_eq!(
        fs::read_dir(directory.path().join("staging"))
            .map(|entries| entries.count())
            .ok(),
        Some(0)
    );
}

#[test]
fn corrupt_hash_properties_and_byte_budget_leave_no_staged_result() {
    for variant in 0..3 {
        let (directory, mut input) = fixture();
        let mut limits = AudioAssemblyLimits::default();
        match variant {
            0 => input.timeline.utterances[0].audio_sha256 = sha(&"0".repeat(64)),
            1 => input.timeline.utterances[0].frame_count = 5,
            _ => limits.max_input_bytes = 1,
        }
        assert!(prepare_case(&directory, &input, &limits).is_err());
        assert_eq!(
            fs::read_dir(directory.path().join("staging"))
                .map(|entries| entries.count())
                .ok(),
            Some(0)
        );
    }
}

#[test]
fn outside_input_root_is_refused_without_modifying_it() {
    let (directory, mut input) = fixture();
    let outside = directory.path().join("outside.wav");
    assert!(fs::copy(&input.inputs[0].path, &outside).is_ok());
    input.inputs[0].path = outside.clone();
    assert!(prepare_case(&directory, &input, &AudioAssemblyLimits::default()).is_err());
    assert!(outside.is_file());
}
