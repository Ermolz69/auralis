use crate::{
    TtsCompleted, TtsExecutionRequest,
    audio_layout::{AUDIO_LAYOUT_POLICY, verify_result_audio_layout_cancellable},
    result_verification::invalid,
    verification_control::{VerificationGuard, check_cancel},
};
use ports::{
    PortError, cancellation::CancellationToken, speech_audio::SpeechAudioLayout,
    speech_workflow::SpeechProducedFile,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path};

#[cfg(test)]
mod tests;

pub(crate) async fn produce(
    request: TtsExecutionRequest,
    completed: TtsCompleted,
    manifest: &Value,
    cancel: CancellationToken,
) -> Result<Option<SpeechProducedFile>, PortError> {
    check_cancel(&cancel)?;
    if manifest.get("audio_assembly").and_then(Value::as_str) != Some(AUDIO_LAYOUT_POLICY) {
        return Ok(None);
    }
    let cancellation = cancel.child_token();
    let guard = VerificationGuard(cancellation.clone());
    let result = tokio::task::spawn_blocking(move || {
        let layout = verify_result_audio_layout_cancellable(&request, &completed, &cancellation)?;
        let path = request.out_file.with_file_name("audio-layout.json");
        write_layout(&layout, &path, &cancellation)
    })
    .await
    .map_err(|error| PortError::Unexpected {
        message: error.to_string(),
    })?;
    drop(guard);
    result.map(Some)
}

fn write_layout(
    layout: &SpeechAudioLayout,
    path: &Path,
    cancel: &CancellationToken,
) -> Result<SpeechProducedFile, PortError> {
    check_cancel(cancel)?;
    let data = serde_json::to_vec(layout).map_err(|error| invalid(&error.to_string()))?;
    if data.len() > ports::speech_run::MAX_SPEECH_PLAN_BYTES {
        return Err(invalid(
            "Serialized audio layout exceeds the metadata limit",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Audio layout requires an owned attempt directory"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    temporary.write_all(&data).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    check_cancel(cancel)?;
    temporary
        .persist_noclobber(path)
        .map_err(|error| io_error(error.error))?;
    check_cancel(cancel)?;
    Ok(SpeechProducedFile {
        kind: "audio_layout".into(),
        path: path.into(),
        sha256: format!("{:x}", Sha256::digest(&data)),
    })
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}
