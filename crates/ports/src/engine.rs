use async_trait::async_trait;

use domain::media::{Artifact, SynthesizedSegment};
use domain::transcript::Transcript;
use domain::dubbing::DubbingConfig;

use crate::error::PortError;

pub struct SynthesizeSpeechRequest {
    pub transcript: Transcript,
    pub config: DubbingConfig,
    pub target_dir: std::path::PathBuf,
}

#[async_trait]
pub trait AsrEnginePort: Send + Sync {
    async fn transcribe_audio(&self, audio_artifact: &Artifact) -> Result<Transcript, PortError>;
}

#[async_trait]
pub trait TtsEnginePort: Send + Sync {
    async fn synthesize_segments(
        &self,
        request: SynthesizeSpeechRequest,
    ) -> Result<Vec<SynthesizedSegment>, PortError>;
}
