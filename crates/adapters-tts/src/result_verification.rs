use crate::execute::{TtsCompleted, TtsExecutionRequest, confined};
use crate::wav_verification::inspect_pcm16_wav;
use ports::error::PortError;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_RESULT_JSON_BYTES: u64 = 16 * 1024 * 1024;

/// Recheck a retained worker result before publication or recovery.
pub fn verify_result_files(
    request: &TtsExecutionRequest,
    completed: &TtsCompleted,
) -> Result<(), PortError> {
    if completed.output != request.out_file
        || completed.manifest != request.manifest_path
        || completed.utterance_count != request.expected_utterances
        || completed.chunk_count != request.expected_chunks
    {
        return Err(invalid("Worker completion targets or counts changed"));
    }
    let plan = read_json(&request.plan_path)?;
    let manifest = read_json(&completed.manifest)?;
    let profile = object(&plan, "profile")?;
    let plan_id = string(&plan, "plan_id")?;
    let engine = string(profile, "engine_id")?;
    let profile_id = string(profile, "profile_id")?;
    let model_revision = string(profile, "model_revision")?;
    if number(&plan, "version")? != 1
        || string(&plan, "project_id")? != request.project_id.to_string()
        || string(&plan, "plan_digest")? != request.plan_digest
        || string(profile, "profile_sha256")? != hash_file(&request.profile_path)?
        || request
            .profile_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            != Some(profile_id)
    {
        return Err(invalid("Speech plan or installed profile identity changed"));
    }
    let utterances = array(&plan, "utterances")?;
    let segments = array(&manifest, "segments")?;
    let speech_plan = object(&manifest, "speech_plan")?;
    let settings = object(&manifest, "settings")?;
    let engine_settings = object(settings, "engine_settings")?;
    let runtime = object(&manifest, "runtime")?;
    let revisions = object(runtime, "model_revisions")?;
    let revision_key = match engine {
        "voxcpm2" => "openbmb/VoxCPM2",
        "espeech" => "ESpeech/ESpeech-TTS-1_RL-V2",
        _ => return Err(invalid("Unsupported result engine")),
    };
    if number(&manifest, "version")? != 4
        || string(&manifest, "input")? != path_string(&request.plan_path)
        || string(&manifest, "output")? != path_string(&request.out_file)
        || string(&manifest, "output_sha256")? != completed.output_sha256
        || string(&manifest, "engine")? != engine
        || string(&manifest, "profile")? != profile_id
        || number(&manifest, "sample_rate")? != completed.sample_rate as u64
        || string(speech_plan, "plan_id")? != plan_id
        || string(speech_plan, "plan_digest")? != request.plan_digest
        || string(revisions, revision_key)? != model_revision
        || string(engine_settings, "text_segmentation")? != "speech_plan_split_v1"
        || utterances.len() != request.expected_utterances
        || segments.len() != request.expected_chunks
    {
        return Err(invalid(
            "Result manifest identity or coverage differs from the plan",
        ));
    }
    let expected_ids: Vec<Value> = utterances
        .iter()
        .map(|utterance| {
            utterance
                .get("utterance_id")
                .cloned()
                .unwrap_or(Value::Null)
        })
        .collect();
    if array(speech_plan, "expected_utterance_ids")? != expected_ids {
        return Err(invalid("Result utterance order differs from the plan"));
    }
    let mut cast = HashMap::new();
    for assignment in array(&plan, "cast")? {
        let role = string(assignment, "role_id")?;
        let voice = string(assignment, "voice_id")?;
        if cast.insert(role, voice).is_some() {
            return Err(invalid("Speech plan repeats a role"));
        }
    }
    let mut cursor = 0;
    for utterance in utterances {
        let id = string(utterance, "utterance_id")?;
        let role = string(utterance, "role_id")?;
        let voice = cast
            .get(role)
            .ok_or_else(|| invalid("Unassigned speech role"))?;
        let mut text = String::new();
        let mut ordinal = 0;
        while let Some(segment) = segments.get(cursor) {
            if string(segment, "utterance_id")? != id {
                break;
            }
            ordinal += 1;
            cursor += 1;
            let audio = PathBuf::from(string(segment, "audio")?);
            confined(&request.cache_dir, &audio)?;
            if string(segment, "kind")? != "text"
                || !matches!(
                    string(segment, "status")?,
                    "generated" | "cached" | "reused_in_job"
                )
                || string(segment, "chunk_id")? != format!("{id}-chunk-{ordinal:04}")
                || string(segment, "voice")? != role
                || string(segment, "resolved_voice")? != *voice
                || string(segment, "reference_id")? != string(utterance, "reference_id")?
                || string(segment, "emotion")? != string(utterance, "emotion_id")?
                || string(segment, "engine")? != engine
                || segment.get("cue_id") != utterance.get("cue_id")
                || segment.get("source_start_ms") != utterance.get("start_ms")
                || segment.get("source_end_ms") != utterance.get("end_ms")
                || float(segment, "speed")? != float(utterance, "speed")?
                || float(segment, "volume")? != float(utterance, "volume")?
                || string(segment, "seed_key")?
                    != format!("{}:chunk:{ordinal}", string(utterance, "seed_key")?)
                || number(segment, "duration_ms")? == 0
                || string(segment, "audio_sha256")? != hash_file(&audio)?
            {
                return Err(invalid("Result chunk differs from its reviewed utterance"));
            }
            text.push_str(string(segment, "text")?);
        }
        if ordinal == 0 || text != string(utterance, "text")? {
            return Err(invalid("Result omits or changes reviewed utterance text"));
        }
    }
    if cursor != segments.len() {
        return Err(invalid("Result contains an extra or reordered utterance"));
    }
    if hash_file(&request.out_file)? != completed.output_sha256
        || hash_file(&request.manifest_path)? != completed.manifest_sha256
    {
        return Err(invalid(
            "Result WAV or manifest hash changed before publication",
        ));
    }
    let wav = inspect_pcm16_wav(&request.out_file)?;
    if wav.sample_rate != completed.sample_rate
        || wav.channels != completed.channels
        || wav.frames != completed.frames
        || wav.duration_ms != completed.duration_ms
    {
        return Err(invalid(
            "Result WAV measurements differ from worker completion",
        ));
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, PortError> {
    if path.is_symlink()
        || !path.is_file()
        || fs::metadata(path).map_err(io_error)?.len() > MAX_RESULT_JSON_BYTES
    {
        return Err(invalid(
            "Speech plan or result manifest is not a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io_error)?
        .take(MAX_RESULT_JSON_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > MAX_RESULT_JSON_BYTES {
        return Err(invalid(
            "Speech plan or result manifest grew beyond its byte limit",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| invalid("Speech plan or result manifest is invalid JSON"))
}

fn object<'a>(value: &'a Value, key: &str) -> Result<&'a Value, PortError> {
    value
        .get(key)
        .filter(|item| item.is_object())
        .ok_or_else(|| invalid("Missing result object"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], PortError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| invalid("Missing result array"))
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, PortError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("Missing result string"))
}

fn number(value: &Value, key: &str) -> Result<u64, PortError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid("Missing result number"))
}

fn float(value: &Value, key: &str) -> Result<f64, PortError> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| invalid("Missing result setting"))
}

pub(crate) fn hash_file(path: &Path) -> Result<String, PortError> {
    if path.is_symlink() || !path.is_file() {
        return Err(invalid("TTS result is not a regular file"));
    }
    let mut stream = File::open(path).map_err(io_error)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = stream.read(&mut buffer).map_err(io_error)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidStoredData {
        entity_type: "tts_result".into(),
        entity_id: "manifest".into(),
        field: "coverage".into(),
        message: message.into(),
    }
}
