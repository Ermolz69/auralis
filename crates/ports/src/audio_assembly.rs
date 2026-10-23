use crate::{PortError, cancellation::CancellationToken};
use async_trait::async_trait;
use domain::speech::{Sha256Digest, SpeechId, timeline::AudioTimelineRequest};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct UtteranceAudioFile {
    pub utterance_id: SpeechId,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AudioAssemblyRequest {
    pub timeline: AudioTimelineRequest,
    pub inputs: Vec<UtteranceAudioFile>,
}

#[derive(Debug, Clone)]
pub struct StagedAudioAssembly {
    pub staging_root: PathBuf,
    pub wav_path: PathBuf,
    pub manifest_path: PathBuf,
    pub wav_sha256: Sha256Digest,
    pub manifest_sha256: Sha256Digest,
    pub pcm_sha256: Sha256Digest,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub peak_sample_abs: u16,
    pub full_scale_samples: u64,
    pub admission_guardian_pid: u32,
}

#[async_trait]
pub trait AudioAssemblyPort: Send + Sync {
    async fn assemble(
        &self,
        request: AudioAssemblyRequest,
        cancellation: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError>;
}
