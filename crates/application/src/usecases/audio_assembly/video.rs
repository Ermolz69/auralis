use super::{
    AudioAssemblyUseCases,
    error::{conflict, invalid},
};
use crate::error::ApplicationError;
use domain::{
    media::{ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState},
    project::ProjectId,
};
use ports::{
    audio_assembly::StagedAudioAssembly,
    audio_jobs::{AssemblySourceArtifact, AudioAssemblyPlan},
    cancellation::CancellationToken,
    video_export::{StagedVideoExport, VideoAssemblySource, VideoExportRequest, VideoSourceInput},
};

impl AudioAssemblyUseCases {
    async fn video_input(
        &self,
        project: &ProjectId,
        id: &ArtifactId,
    ) -> Result<VideoSourceInput, ApplicationError> {
        let artifact = self
            .index
            .get(id)
            .await?
            .ok_or_else(|| invalid("Source video is not ready"))?;
        let ArtifactLocation::StorageKey(key) = &artifact.location else {
            return Err(invalid("Source video must be managed").into());
        };
        if !key.starts_with(&format!("{project}/"))
            || artifact.state != ArtifactState::Ready
            || !matches!(
                artifact.kind,
                ArtifactKind::SourceVideo | ArtifactKind::DownloadedVideo
            )
        {
            return Err(conflict("Source video belongs to another project or is not ready").into());
        }
        let size_bytes = artifact
            .size_bytes
            .filter(|size| *size > 0 && *size <= 1_u64 << 32)
            .ok_or_else(|| invalid("Invalid source video size"))?;
        Ok(VideoSourceInput {
            path: self.artifacts.resolve_artifact(&artifact).await?,
            size_bytes,
            expected_sha256: None,
        })
    }

    pub(super) async fn inspect_video(
        &self,
        project: &ProjectId,
        id: ArtifactId,
        cancel: CancellationToken,
    ) -> Result<VideoAssemblySource, ApplicationError> {
        let input = self.video_input(project, &id).await?;
        let media = self
            .video_export
            .as_ref()
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Экспорт видео недоступен. Восстановите установленные FFmpeg и FFprobe."
                    .into(),
            })?
            .inspect(input, cancel)
            .await?;
        Ok(VideoAssemblySource {
            policy_id: ports::video_export::REPLACE_AUDIO_POLICY.into(),
            artifact: AssemblySourceArtifact {
                artifact_id: id,
                sha256: media.sha256.clone(),
                size_bytes: media.size_bytes,
            },
            media,
        })
    }

    pub(super) async fn render_video(
        &self,
        plan: &AudioAssemblyPlan,
        result: &StagedAudioAssembly,
        cancel: CancellationToken,
    ) -> Result<Option<StagedVideoExport>, ApplicationError> {
        let Some(video) = &plan.video else {
            return Ok(None);
        };
        let mut source = self
            .video_input(&plan.project_id, &video.artifact.artifact_id)
            .await?;
        if source.size_bytes != video.artifact.size_bytes {
            return Err(conflict("Reviewed video size changed").into());
        }
        source.expected_sha256 = Some(video.artifact.sha256.clone());
        let plan_json = serde_json::to_vec(plan).map_err(|error| invalid(error.to_string()))?;
        use sha2::{Digest, Sha256};
        let hash = domain::speech::Sha256Digest::parse(format!("{:x}", Sha256::digest(&plan_json)))
            .map_err(invalid)?;
        self.video_export
            .as_ref()
            .ok_or_else(|| invalid("Managed video exporter is unavailable"))?
            .export(
                VideoExportRequest {
                    policy_id: video.policy_id.clone(),
                    source,
                    reviewed: video.media.clone(),
                    wav_path: result.wav_path.clone(),
                    wav_sha256: result.wav_sha256.clone(),
                    frames: result.frames,
                    channels: result.channels,
                    output_directory: result.staging_root.clone(),
                    assembly_plan_sha256: hash,
                },
                cancel,
            )
            .await
            .map(Some)
            .map_err(Into::into)
    }
}
