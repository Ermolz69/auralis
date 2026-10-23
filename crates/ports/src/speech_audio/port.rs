use super::SpeechAudioAssemblyRequest;
use crate::{PortError, audio_assembly::StagedAudioAssembly, cancellation::CancellationToken};
use async_trait::async_trait;

#[async_trait]
pub trait SpeechAudioAssemblyPort: Send + Sync {
    async fn assemble_speech(
        &self,
        request: SpeechAudioAssemblyRequest,
        cancellation: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError>;
}
