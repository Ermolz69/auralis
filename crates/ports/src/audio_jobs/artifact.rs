use domain::{media::ArtifactId, speech::Sha256Digest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssemblySourceArtifact {
    pub artifact_id: ArtifactId,
    pub sha256: Sha256Digest,
    pub size_bytes: u64,
}
