use super::SpeechAudioAssemblyRequest;
use crate::{PortError, audio_assembly::StagedAudioAssembly, cancellation::CancellationToken};
use async_trait::async_trait;

#[async_trait]
pub trait SpeechAudioAssemblyPort: Send + Sync {
    async fn release_staged(&self, _root: std::path::PathBuf) -> Result<(), PortError> {
        Err(PortError::Unsupported {
            message: "Assembly staging cleanup is not supported".into(),
        })
    }
    async fn assemble_speech(
        &self,
        request: SpeechAudioAssemblyRequest,
        cancellation: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError>;
}
