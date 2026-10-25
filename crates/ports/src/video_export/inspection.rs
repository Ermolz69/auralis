use domain::speech::Sha256Digest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoInspection {
    pub sha256: Sha256Digest,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub ffprobe_sha256: Sha256Digest,
}
