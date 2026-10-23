use domain::speech::{
    Sha256Digest, SpeechId, TimingMode,
    timeline::{AudioTimelineRequest, MeasuredUtterance, SourceWindow},
};
use ports::audio_assembly::{AudioAssemblyRequest, UtteranceAudioFile};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub version: u8,
    pub source_commit: String,
    pub generator_sha256: String,
    pub node_version: String,
    pub ffmpeg: Tool,
    pub fixture_root: String,
    pub fixtures: Vec<Fixture>,
    pub budgets: Budgets,
    pub cases: Vec<Case>,
    pub scope: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub file: String,
    pub sha256: String,
    pub version: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub file: String,
    pub sha256: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    pub max_invocations_per_case: u32,
    pub process_seconds: u64,
    pub total_seconds: u64,
    pub cleanup_seconds: u64,
    pub max_generated_bytes: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub mode: TimingMode,
    pub fixture_indices: Vec<usize>,
    pub pauses_ms: Vec<u64>,
    pub windows: Vec<Option<SourceWindow>>,
    pub source_duration_ms: Option<u64>,
    pub expected_frames: u64,
    pub expected_pcm_sha256: String,
    pub expected_peak: u16,
    pub expected_full_scale: u64,
}

pub fn hash_file(path: &Path) -> String {
    let mut file = match File::open(path) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = match file.read(&mut buffer) {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        };
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    format!("{:x}", hash.finalize())
}
pub fn digest(value: &str) -> Sha256Digest {
    match Sha256Digest::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

pub fn request(spec: &Spec, case: &Case, fixtures: &Path, spec_hash: &str) -> AudioAssemblyRequest {
    assert_eq!(case.fixture_indices.len(), case.pauses_ms.len());
    assert_eq!(case.fixture_indices.len(), case.windows.len());
    let mut facts = Vec::new();
    let mut inputs = Vec::new();
    for (index, fixture_index) in case.fixture_indices.iter().enumerate() {
        let fixture = &spec.fixtures[*fixture_index];
        let id = match SpeechId::parse(format!("{}-u-{index:04}", case.case_id)) {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        };
        facts.push(MeasuredUtterance {
            utterance_id: id.clone(),
            audio_sha256: digest(&fixture.sha256),
            sample_rate: fixture.sample_rate,
            channels: fixture.channels,
            frame_count: fixture.frames,
            source_window: case.windows[index].clone(),
            pause_after_ms: case.pauses_ms[index],
        });
        inputs.push(UtteranceAudioFile {
            utterance_id: id,
            path: fixtures.join(&fixture.file),
        });
    }
    AudioAssemblyRequest {
        timeline: AudioTimelineRequest {
            source_plan_sha256: digest(spec_hash),
            raw_manifest_sha256: digest(&format!(
                "{:x}",
                Sha256::digest(format!("fixture-raw:{spec_hash}:{}", case.case_id))
            )),
            mode: case.mode,
            expected_utterance_ids: facts.iter().map(|item| item.utterance_id.clone()).collect(),
            utterances: facts,
            source_duration_ms: case.source_duration_ms,
        },
        inputs,
    }
}
