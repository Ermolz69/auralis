use domain::speech::Sha256Digest;
use std::path::PathBuf;

pub struct StagedVideoExport {
    pub video_path: PathBuf,
    pub manifest_path: PathBuf,
    pub video_sha256: Sha256Digest,
    pub manifest_sha256: Sha256Digest,
}
