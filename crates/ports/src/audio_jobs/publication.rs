use crate::speech_workflow::SpeechPublishedFile;
use domain::speech::Sha256Digest;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioAssemblyPublication {
    pub assembly_id: Uuid,
    pub attempt_id: Uuid,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub pcm_sha256: Sha256Digest,
    pub peak_sample_abs: u16,
    pub full_scale_samples: u64,
    pub files: Vec<SpeechPublishedFile>,
    pub ready: bool,
}
