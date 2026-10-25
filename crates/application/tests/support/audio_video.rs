use super::*;
use async_trait::async_trait;
use domain::speech::Sha256Digest;
use ports::PortError;
use ports::video_export::{
    StagedVideoExport, VideoExportPort, VideoExportRequest, VideoInspection, VideoSourceInput,
};
use std::sync::Arc;
#[path = "../../../adapters-storage/tests/support/video_source.rs"]
mod video_source;

struct VideoFixture;
#[async_trait]
impl VideoExportPort for VideoFixture {
    async fn inspect(
        &self,
        source: VideoSourceInput,
        _: CancellationToken,
    ) -> Result<VideoInspection, PortError> {
        let bytes = std::fs::read(source.path).map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        Ok(VideoInspection {
            sha256: Sha256Digest::parse(source::digest(&bytes)).map_err(|message| {
                PortError::InvalidSource {
                    message: message.into(),
                }
            })?,
            size_bytes: bytes.len() as u64,
            duration_ms: 5000,
            width: 320,
            height: 180,
            video_codec: "synthetic".into(),
            ffprobe_sha256: source::pin().map_err(|error| PortError::Unexpected {
                message: error.to_string(),
            })?,
        })
    }
    async fn export(
        &self,
        request: VideoExportRequest,
        cancel: CancellationToken,
    ) -> Result<StagedVideoExport, PortError> {
        if cancel.is_cancelled() {
            return Err(PortError::Cancelled);
        }
        let observed = self.inspect(request.source, cancel).await?;
        if observed != request.reviewed {
            return Err(PortError::InvalidSource {
                message: "Synthetic video changed".into(),
            });
        }
        let video = request.output_directory.join("synthetic.mp4");
        let manifest = request.output_directory.join("synthetic-video.json");
        std::fs::write(&video, b"Synthetic codec fixture").map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        std::fs::write(&manifest, b"{}").map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        Ok(StagedVideoExport {
            video_path: video,
            manifest_path: manifest,
            video_sha256: Sha256Digest::parse(source::digest(b"Synthetic codec fixture")).map_err(
                |message| PortError::InvalidSource {
                    message: message.into(),
                },
            )?,
            manifest_sha256: Sha256Digest::parse(source::digest(b"{}")).map_err(|message| {
                PortError::InvalidSource {
                    message: message.into(),
                }
            })?,
        })
    }
}

#[tokio::test]
async fn video_job_binds_the_source_and_publishes_four_verified_files() -> TestResult {
    let mut fixture = Fixture::new().await?;
    fixture.cases.video_export = Some(Arc::new(VideoFixture));
    let video = video_source::video_source(&fixture.source).await?;
    let mut request = fixture.request(0);
    request.video_artifact_id = Some(video.artifact.artifact_id.clone());
    let preview = fixture
        .cases
        .inspect(
            fixture.source.project_id.clone(),
            request.clone(),
            CancellationToken::new(),
        )
        .await?;
    assert_eq!(
        preview.video.as_ref().ok_or("Missing video")?.media.sha256,
        video.media.sha256
    );
    let prepared = fixture
        .cases
        .prepare(
            fixture.source.project_id.clone(),
            PrepareAssemblyRequest {
                source: request,
                expected_preview_sha256: preview.preview_sha256,
                reviewed: true,
            },
        )
        .await?;
    fixture
        .cases
        .start(
            fixture.source.project_id.clone(),
            prepared.assembly_id,
            false,
        )
        .await?;
    let pending = fixture.publication(prepared.assembly_id).await?;
    assert!(!pending.ready);
    for file in &pending.files {
        assert!(fixture.source.index.get(&file.artifact_id).await?.is_none());
    }
    let completed = fixture.completed(prepared.assembly_id).await?;
    assert!(completed.video.is_some());
    let publication = completed.result.ok_or("Missing completed result")?;
    assert_eq!(publication.files.len(), 4);
    assert_eq!(
        publication
            .files
            .iter()
            .filter(|file| file.kind == "video")
            .count(),
        1
    );
    fixture
        .cases
        .verify(fixture.source.project_id.clone(), prepared.assembly_id)
        .await?;
    fixture.close().await
}

#[tokio::test]
async fn changed_video_bytes_invalidate_the_exact_preview_without_a_job() -> TestResult {
    let mut fixture = Fixture::new().await?;
    fixture.cases.video_export = Some(Arc::new(VideoFixture));
    let video = video_source::video_source(&fixture.source).await?;
    let mut request = fixture.request(0);
    request.video_artifact_id = Some(video.artifact.artifact_id.clone());
    let preview = fixture
        .cases
        .inspect(
            fixture.source.project_id.clone(),
            request.clone(),
            CancellationToken::new(),
        )
        .await?;
    let artifact = fixture
        .source
        .index
        .get(&video.artifact.artifact_id)
        .await?
        .ok_or("Missing video")?;
    let path = fixture.source.files.resolve_artifact(&artifact).await?;
    let mut changed = std::fs::read(&path)?;
    changed[0] ^= 1;
    std::fs::write(path, changed)?;
    assert!(
        fixture
            .cases
            .prepare(
                fixture.source.project_id.clone(),
                PrepareAssemblyRequest {
                    source: request,
                    expected_preview_sha256: preview.preview_sha256,
                    reviewed: true
                }
            )
            .await
            .is_err()
    );
    fixture.close().await
}

#[tokio::test]
async fn overlong_video_speech_explains_the_timing_conflict_before_preparation() -> TestResult {
    let mut fixture = Fixture::new().await?;
    fixture.cases.video_export = Some(Arc::new(VideoFixture));
    let video = video_source::video_source(&fixture.source).await?;
    let mut request = fixture.request(6000);
    request.video_artifact_id = Some(video.artifact.artifact_id);
    let error = fixture
        .cases
        .inspect(
            fixture.source.project_id.clone(),
            request,
            CancellationToken::new(),
        )
        .await
        .err()
        .ok_or("Expected timing refusal")?;
    let application::error::ApplicationError::InvalidOperation { message } = error else {
        return Err("Expected actionable timing conflict".into());
    };
    assert!(message.contains("6100 мс > 5000 мс"));
    assert_eq!(
        fixture
            .renderer
            .calls
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    fixture.close().await
}
