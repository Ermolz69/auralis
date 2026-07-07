use async_trait::async_trait;
use std::path::{Path, PathBuf};

use domain::media::MediaMetadata;
use ports::error::PortError;
use ports::media::MediaProbePort;

use crate::ffprobe::command::run_ffprobe;
use crate::ffprobe::parser::parse_ffprobe_output;

#[derive(Debug, Clone)]
pub struct FfprobeAdapter {
    candidates: Vec<PathBuf>,
}

impl Default for FfprobeAdapter {
    fn default() -> Self {
        Self::new(vec![PathBuf::from("ffprobe")])
    }
}

impl FfprobeAdapter {
    pub fn new(candidates: Vec<PathBuf>) -> Self {
        Self { candidates }
    }
}

#[async_trait]
impl MediaProbePort for FfprobeAdapter {
    async fn probe_local_file(&self, path: &Path) -> Result<MediaMetadata, PortError> {
        let output = run_ffprobe(&self.candidates, path).await?;
        let metadata = parse_ffprobe_output(&output)?;
        Ok(metadata)
    }
}
