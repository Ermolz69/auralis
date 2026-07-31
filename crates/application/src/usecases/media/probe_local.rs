#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::PathBuf;

use domain::media::MediaMetadata;
use ports::media::MediaProbePort;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct ProbeLocalMediaRequest {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ProbeLocalMediaResponse {
    pub metadata: MediaMetadata,
}

pub struct ProbeLocalMediaUseCase<P: MediaProbePort> {
    media_probe: P,
}

impl<P: MediaProbePort> ProbeLocalMediaUseCase<P> {
    pub fn new(media_probe: P) -> Self {
        Self { media_probe }
    }

    pub async fn execute(
        &self,
        request: ProbeLocalMediaRequest,
    ) -> Result<ProbeLocalMediaResponse, ApplicationError> {
        if !request.path.exists() || !request.path.is_file() {
            return Err(ApplicationError::InvalidOperation {
                message: format!("Path does not exist or is not a file: {:?}", request.path),
            });
        }

        let metadata = self.media_probe.probe_local_file(&request.path).await?;

        Ok(ProbeLocalMediaResponse { metadata })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_ffmpeg::mock::MockMediaProbeAdapter;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_probe_local_media_success() {
        let probe = MockMediaProbeAdapter::new();
        let use_case = ProbeLocalMediaUseCase::new(probe);

        // Create a dummy temp file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("video.mkv");
        File::create(&file_path).unwrap();

        let request = ProbeLocalMediaRequest { path: file_path };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.metadata.duration_ms, 5000);
    }

    #[tokio::test]
    async fn test_probe_local_media_not_found() {
        let probe = MockMediaProbeAdapter::new();
        let use_case = ProbeLocalMediaUseCase::new(probe);

        let request = ProbeLocalMediaRequest {
            path: PathBuf::from("/non/existent/path.mp4"),
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::InvalidOperation { .. }));
    }
}
