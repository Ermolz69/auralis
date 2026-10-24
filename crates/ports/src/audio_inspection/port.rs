use super::{SpeechAudioInspection, SpeechAudioInspectionRequest};
use crate::{PortError, cancellation::CancellationToken};
use async_trait::async_trait;

#[async_trait]
pub trait SpeechAudioInspectionPort: Send + Sync {
    async fn inspect(
        &self,
        request: SpeechAudioInspectionRequest,
        cancel: CancellationToken,
    ) -> Result<SpeechAudioInspection, PortError>;
}
