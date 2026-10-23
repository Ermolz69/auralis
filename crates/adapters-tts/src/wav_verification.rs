use common::wav::{Pcm16Error, WavProperties};
use ports::error::PortError;
use std::path::Path;

pub fn inspect_pcm16_wav(path: &Path) -> Result<WavProperties, PortError> {
    common::wav::inspect_pcm16_wav(path).map_err(|error| match error {
        Pcm16Error::Io(error) => PortError::Io {
            message: error.to_string(),
        },
        Pcm16Error::Invalid(message) => PortError::InvalidStoredData {
            entity_type: "tts_result".into(),
            entity_id: "wav".into(),
            field: "audio".into(),
            message: message.into(),
        },
    })
}
