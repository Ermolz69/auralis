use super::*;

impl TtsExecutionRequest {
    pub(super) fn validate(&self, client: &TtsWorkerClient) -> Result<(), PortError> {
        if !portable_id(&self.run_id)
            || !portable_id(&self.attempt_id)
            || !hex_sha256(&self.plan_digest)
            || self.expected_utterances == 0
            || self.expected_chunks < self.expected_utterances
        {
            return Err(invalid_event(
                "Invalid TTS execution identity or expected coverage",
            ));
        }
        let runtime = client.runtime();
        if !runtime.python_executable.is_file()
            || !runtime.voice_store.is_dir()
            || !runtime.managed_root.is_dir()
            || !runtime.profile_root.is_dir()
            || !self.plan_path.is_file()
            || !self.profile_path.is_file()
        {
            return Err(PortError::Unsupported {
                message: "TTS runtime or inputs are unavailable".into(),
            });
        }
        for path in [
            &self.plan_path,
            &self.out_file,
            &self.manifest_path,
            &self.cache_dir,
            &self.checkpoint_root,
        ] {
            confined(&runtime.managed_root, path)?;
        }
        confined(&runtime.profile_root, &self.profile_path)?;
        Ok(())
    }
}

impl SavedCheckpoint {
    pub(super) fn validate(
        &self,
        request: &TtsExecutionRequest,
        expected_sequence: usize,
    ) -> Result<(), PortError> {
        if self.sequence != expected_sequence as u64
            || self.run_id != request.run_id
            || self.attempt_id != request.attempt_id
            || self.plan_digest != request.plan_digest
            || !portable_id(&self.utterance_id)
            || !portable_id(&self.chunk_id)
            || !hex_sha256(&self.audio_sha256)
            || !hex_sha256(&self.cache_key)
            || self.duration_ms == 0
            || !matches!(
                self.status.as_str(),
                "generated" | "cached" | "reused_in_job"
            )
            || expected_sequence > request.expected_chunks
        {
            return Err(invalid_event("Invalid or stale saved TTS checkpoint"));
        }
        Ok(())
    }
}

impl TtsCompleted {
    pub(super) fn validate(
        &self,
        request: &TtsExecutionRequest,
        checkpoints: usize,
    ) -> Result<(), PortError> {
        if self.output != request.out_file
            || self.manifest != request.manifest_path
            || !hex_sha256(&self.output_sha256)
            || !hex_sha256(&self.manifest_sha256)
            || self.sample_rate == 0
            || self.channels == 0
            || self.frames == 0
            || self.duration_ms == 0
            || self.utterance_count != request.expected_utterances
            || self.chunk_count != request.expected_chunks
            || checkpoints != request.expected_chunks
        {
            return Err(invalid_event("Incomplete or mismatched TTS completion"));
        }
        Ok(())
    }
}

pub(crate) fn confined(root: &Path, path: &Path) -> Result<(), PortError> {
    if !path.is_absolute()
        || !path.starts_with(root)
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(invalid_event("TTS path escapes its managed root"));
    }
    let mut current = root.to_path_buf();
    if current.is_symlink() {
        return Err(invalid_event("TTS managed root is a link"));
    }
    for component in path
        .strip_prefix(root)
        .map_err(|_| invalid_event("TTS path escapes its managed root"))?
        .components()
    {
        current.push(component);
        if current.is_symlink() {
            return Err(invalid_event("TTS path contains a link"));
        }
    }
    Ok(())
}

fn portable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}

pub(super) fn unexpected(message: &str) -> PortError {
    PortError::ExternalToolFailed {
        tool: "puza-tts-worker".into(),
        message: message.into(),
    }
}

pub(super) fn invalid_event(message: impl Into<String>) -> PortError {
    PortError::InvalidStoredData {
        entity_type: "tts_worker".into(),
        entity_id: "event".into(),
        field: "event".into(),
        message: message.into(),
    }
}
