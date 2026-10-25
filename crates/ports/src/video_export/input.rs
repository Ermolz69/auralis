use domain::speech::Sha256Digest;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VideoSourceInput {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub expected_sha256: Option<Sha256Digest>,
}
