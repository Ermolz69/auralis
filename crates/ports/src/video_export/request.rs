use super::{VideoInspection, VideoSourceInput};
use domain::speech::Sha256Digest;
use std::path::PathBuf;

pub struct VideoExportRequest {
    pub policy_id: String,
    pub source: VideoSourceInput,
    pub reviewed: VideoInspection,
    pub wav_path: PathBuf,
    pub wav_sha256: Sha256Digest,
    pub frames: u64,
    pub channels: u16,
    pub output_directory: PathBuf,
    pub assembly_plan_sha256: Sha256Digest,
}
