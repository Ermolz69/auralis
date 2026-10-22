use ports::{
    error::PortError,
    speech_run::{
        MAX_SPEECH_CHUNKS, MAX_SPEECH_PLAN_BYTES, MAX_SPEECH_UTTERANCES, SpeechRunIntent,
    },
};
use serde::Deserialize;

// These host admission ceilings are independent of voice qualification and
// retain the worker's existing bounded plan/result protocol.
pub const MAX_SPEECH_SOURCE_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpeechLimits {
    version: u8,
    max_utterances: u32,
    max_chunks: u32,
    max_source_bytes: usize,
}

impl Default for SpeechLimits {
    fn default() -> Self {
        Self {
            version: 1,
            max_utterances: 4096,
            max_chunks: MAX_SPEECH_CHUNKS,
            max_source_bytes: MAX_SPEECH_SOURCE_BYTES,
        }
    }
}

impl SpeechLimits {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let limits: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if limits.version != 1
            || !(1..=MAX_SPEECH_UTTERANCES).contains(&limits.max_utterances)
            || !(limits.max_utterances..=MAX_SPEECH_CHUNKS).contains(&limits.max_chunks)
            || !(1..=MAX_SPEECH_SOURCE_BYTES).contains(&limits.max_source_bytes)
        {
            return Err(format!(
                "version must be 1; max_utterances must be 1..={MAX_SPEECH_UTTERANCES}; \
                 max_chunks must be max_utterances..={MAX_SPEECH_CHUNKS}; \
                 max_source_bytes must be 1..={MAX_SPEECH_SOURCE_BYTES}"
            ));
        }
        Ok(limits)
    }

    pub(super) fn source(&self, text: &str, cues: usize) -> Result<(), PortError> {
        if text.len() > self.max_source_bytes || cues > self.max_utterances as usize {
            return Err(invalid(format!(
                "Speech source has {cues} cues / {} UTF-8 bytes; configured limits are {} cues / {} bytes (speech-limits.json)",
                text.len(),
                self.max_utterances,
                self.max_source_bytes
            )));
        }
        Ok(())
    }

    pub(super) fn plan(&self, intent: &SpeechRunIntent) -> Result<(), PortError> {
        if intent.expected_utterances == 0
            || intent.expected_utterances > self.max_utterances
            || intent.expected_chunks < intent.expected_utterances
            || intent.expected_chunks > self.max_chunks
            || intent.plan_json.len() > MAX_SPEECH_PLAN_BYTES
        {
            return Err(invalid(format!(
                "Speech plan has {} utterances / {} chunks / {} bytes; configured limits are {} utterances / {} chunks / {MAX_SPEECH_PLAN_BYTES} bytes (speech-limits.json)",
                intent.expected_utterances,
                intent.expected_chunks,
                intent.plan_json.len(),
                self.max_utterances,
                self.max_chunks
            )));
        }
        Ok(())
    }
}

fn invalid(message: String) -> PortError {
    PortError::InvalidSource { message }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn lower_operator_limits_are_enforced_in_utf8_bytes() {
        let limits = SpeechLimits::from_json(
            br#"{"version":1,"max_utterances":600,"max_chunks":1200,"max_source_bytes":6}"#,
        )
        .unwrap();
        assert!(limits.source("你好", 600).is_ok());
        assert!(limits.source("你好啊", 600).is_err());
        assert!(limits.source("好", 601).is_err());
        assert!(SpeechLimits::default().source("长故事", 601).is_ok());
    }

    #[test]
    fn invalid_configuration_never_silently_becomes_a_default() {
        for bytes in [
            r#"{"version":2}"#,
            r#"{"max_utterances":0}"#,
            r#"{"max_utterances":8193}"#,
            r#"{"max_utterances":600,"max_chunks":599}"#,
            r#"{"max_chunks":8193}"#,
            r#"{"max_source_bytes":1048577}"#,
            r#"{"max_source_bytes":0}"#,
            r#"{"max_utterance":600}"#,
        ] {
            assert!(
                SpeechLimits::from_json(bytes.as_bytes()).is_err(),
                "{bytes}"
            );
        }
    }
}
