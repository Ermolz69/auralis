use adapters_tts::{TtsCompleted, TtsExecutionRequest, verify_result_files};
use domain::project::ProjectId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    request: TtsExecutionRequest,
    completed: TtsCompleted,
    manifest: Value,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let project_id = ProjectId::new();
        let profile_path = root.join("native-plan-voxcpm2.json");
        let plan_path = root.join("plan.json");
        let out_file = root.join("result.wav");
        let manifest_path = root.join("result.manifest.json");
        let cache_dir = root.join("cache");
        fs::create_dir(&cache_dir)?;
        let chunk_audio = cache_dir.join("chunk.wav");
        fs::write(&profile_path, b"pinned-profile")?;
        fs::write(&out_file, minimal_wav())?;
        fs::write(&chunk_audio, minimal_wav())?;
        let plan_digest = "a".repeat(64);
        let source = json!({
            "version": 1,
            "plan_id": "plan-1",
            "project_id": project_id.to_string(),
            "plan_digest": plan_digest,
            "profile": {
                "profile_id": "native-plan-voxcpm2",
                "profile_sha256": digest(&profile_path)?,
                "engine_id": "voxcpm2",
                "model_revision": "32279effe8c19989596f05d353d1447f51d9e915"
            },
            "cast": [{"role_id": "narrator", "voice_id": "marina_elevenlabs_ru"}],
            "utterances": [{
                "utterance_id": "line-1", "cue_id": "cue-1", "text": "Hello world.",
                "role_id": "narrator", "reference_id": "neutral-1", "emotion_id": "neutral",
                "start_ms": 100, "end_ms": 2000, "speed": 1.0, "volume": 1.0,
                "seed_key": "seed-1"
            }]
        });
        fs::write(&plan_path, serde_json::to_vec(&source)?)?;
        let manifest = json!({
            "version": 4,
            "input": plan_path.to_string_lossy(),
            "output": out_file.to_string_lossy(),
            "output_sha256": digest(&out_file)?,
            "engine": "voxcpm2", "profile": "native-plan-voxcpm2", "sample_rate": 48000,
            "runtime": {"model_revisions": {"openbmb/VoxCPM2": "32279effe8c19989596f05d353d1447f51d9e915"}},
            "settings": {"engine_settings": {"text_segmentation": "speech_plan_split_v1"}},
            "speech_plan": {"plan_id": "plan-1", "plan_digest": plan_digest, "expected_utterance_ids": ["line-1"]},
            "segments": [{
                "utterance_id": "line-1", "cue_id": "cue-1", "chunk_id": "line-1-chunk-0001",
                "kind": "text", "status": "generated", "text": "Hello world.",
                "voice": "narrator", "resolved_voice": "marina_elevenlabs_ru", "reference_id": "neutral-1",
                "emotion": "neutral", "engine": "voxcpm2", "source_start_ms": 100,
                "source_end_ms": 2000, "speed": 1.0, "volume": 1.0,
                "seed_key": "seed-1:chunk:1", "duration_ms": 1000,
                "audio": chunk_audio.to_string_lossy(), "audio_sha256": digest(&chunk_audio)?
            }]
        });
        fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
        let completed = TtsCompleted {
            output: out_file.clone(),
            output_sha256: digest(&out_file)?,
            manifest: manifest_path.clone(),
            manifest_sha256: digest(&manifest_path)?,
            sample_rate: 48000,
            channels: 1,
            frames: 48000,
            duration_ms: 1000,
            utterance_count: 1,
            chunk_count: 1,
        };
        let request = TtsExecutionRequest {
            project_id,
            run_id: "run-1".into(),
            attempt_id: "attempt-1".into(),
            plan_digest,
            expected_utterances: 1,
            expected_chunks: 1,
            plan_path,
            profile_path,
            out_file,
            manifest_path,
            cache_dir,
            checkpoint_root: root.join("checkpoints"),
            rebuild: false,
        };
        Ok(Self {
            _temp: temp,
            request,
            completed,
            manifest,
        })
    }

    fn save_manifest(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(
            &self.request.manifest_path,
            serde_json::to_vec(&self.manifest)?,
        )?;
        self.completed.manifest_sha256 = digest(&self.request.manifest_path)?;
        Ok(())
    }
}

fn digest(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn minimal_wav() -> Vec<u8> {
    let frames = 48_000u32;
    let data_bytes = frames * 2;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&48_000u32.to_le_bytes());
    bytes.extend_from_slice(&96_000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    bytes.resize(44 + data_bytes as usize, 0);
    bytes
}

#[test]
fn verifies_retained_result_and_rejects_changed_cue_lineage()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new()?;
    verify_result_files(&fixture.request, &fixture.completed)?;
    fixture.manifest["segments"][0]["cue_id"] = json!("another-cue");
    fixture.save_manifest()?;
    assert!(verify_result_files(&fixture.request, &fixture.completed).is_err());
    Ok(())
}

#[test]
fn rejects_changed_chunk_audio_even_with_unchanged_final_wav()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let chunk = fixture.request.cache_dir.join("chunk.wav");
    fs::write(chunk, b"changed")?;
    assert!(verify_result_files(&fixture.request, &fixture.completed).is_err());
    Ok(())
}

#[test]
fn rejects_non_wav_output_even_when_worker_hashes_match() -> Result<(), Box<dyn std::error::Error>>
{
    let mut fixture = Fixture::new()?;
    fs::write(&fixture.request.out_file, b"not a WAV file")?;
    fixture.completed.output_sha256 = digest(&fixture.request.out_file)?;
    fixture.manifest["output_sha256"] = json!(fixture.completed.output_sha256);
    fixture.save_manifest()?;
    assert!(verify_result_files(&fixture.request, &fixture.completed).is_err());
    Ok(())
}
