use super::VideoInspection;
use crate::audio_jobs::AssemblySourceArtifact;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoAssemblySource {
    pub policy_id: String,
    pub artifact: AssemblySourceArtifact,
    pub media: VideoInspection,
}
