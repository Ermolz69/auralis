use domain::speech::Sha256Digest;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ManagedAudioInput {
    pub path: PathBuf,
    pub sha256: Sha256Digest,
    pub size_bytes: u64,
}
