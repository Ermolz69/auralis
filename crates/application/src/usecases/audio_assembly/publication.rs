use super::{
    AudioAssemblyUseCases,
    error::{conflict, invalid},
};
use crate::error::ApplicationError;
use domain::{job::Job, media::ArtifactKind};
use ports::{
    PortError,
    audio_assembly::StagedAudioAssembly,
    audio_jobs::{AudioAssemblyIntent, AudioAssemblyPublication},
    cancellation::CancellationToken,
    job_scheduler::JobLifecycleEventKind,
    speech_workflow::SpeechPublishedFile,
    storage::{StagedArtifact, VerifiedArtifactCopy},
};
use uuid::Uuid;

impl AudioAssemblyUseCases {
    pub(super) async fn publish_result(
        &self,
        intent: &AudioAssemblyIntent,
        attempt: Uuid,
        job: &Job,
        result: StagedAudioAssembly,
        video: Option<ports::video_export::StagedVideoExport>,
        cancel: CancellationToken,
    ) -> Result<(), ApplicationError> {
        let mut staged = Vec::new();
        let mut files = Vec::new();
        let renderer_root = result.staging_root.clone();
        let mut outputs = vec![
            (
                "wav",
                ArtifactKind::MixedAudio,
                result.wav_path,
                result.wav_sha256,
                1_u64 << 32,
                "assembled.wav",
            ),
            (
                "manifest",
                ArtifactKind::SpeechManifest,
                result.manifest_path,
                result.manifest_sha256,
                16 * 1024 * 1024,
                "assembly.json",
            ),
        ];
        if let Some(video) = video {
            outputs.extend([
                (
                    "video",
                    ArtifactKind::FinalVideo,
                    video.video_path,
                    video.video_sha256,
                    1_u64 << 32,
                    "russian.mp4",
                ),
                (
                    "video_manifest",
                    ArtifactKind::SpeechManifest,
                    video.manifest_path,
                    video.manifest_sha256,
                    16 * 1024 * 1024,
                    "video-export.json",
                ),
            ]);
        }
        for (role, kind, path, hash, cap, name) in outputs {
            if cancel.is_cancelled() {
                self.cleanup_unpublished(&staged, Some(renderer_root))
                    .await?;
                return Err(PortError::Cancelled.into());
            }
            let artifact = match self
                .artifacts
                .stage_verified_copy(
                    VerifiedArtifactCopy {
                        project_id: intent.project_id.clone(),
                        kind,
                        source_path: path,
                        filename_hint: Some(name.into()),
                        expected_sha256: hash.clone(),
                        max_bytes: cap,
                    },
                    cancel.clone(),
                )
                .await
            {
                Ok(artifact) => artifact,
                Err(error) => {
                    self.cleanup_unpublished(&staged, Some(renderer_root))
                        .await?;
                    return Err(error.into());
                }
            };
            files.push(SpeechPublishedFile {
                artifact_id: artifact.artifact.id.clone(),
                kind: role.into(),
                sha256: hash.as_str().into(),
                size_bytes: artifact.size_bytes,
            });
            staged.push(artifact);
        }
        if let Err(error) = self
            .renderer
            .as_ref()
            .ok_or_else(|| invalid("Assembly renderer disappeared"))?
            .release_staged(renderer_root)
            .await
        {
            self.cleanup_unpublished(&staged, None).await?;
            return Err(error.into());
        }
        if cancel.is_cancelled() {
            self.cleanup_unpublished(&staged, None).await?;
            return Err(PortError::Cancelled.into());
        }
        let publication = AudioAssemblyPublication {
            assembly_id: intent.assembly_id,
            attempt_id: attempt,
            sample_rate: result.sample_rate,
            channels: result.channels,
            frames: result.frames,
            pcm_sha256: result.pcm_sha256,
            peak_sample_abs: result.peak_sample_abs,
            full_scale_samples: result.full_scale_samples,
            files,
            ready: false,
        };
        if let Err(error) = self.store.publish(job, &publication, &staged).await {
            if self.store.publication(intent.assembly_id).await?.is_none() {
                self.cleanup_unpublished(&staged, None).await?;
            }
            return Err(error.into());
        }
        for _ in 0..120 {
            if self
                .store
                .publication(intent.assembly_id)
                .await?
                .is_some_and(|publication| publication.ready)
            {
                return Ok(());
            }
            if cancel.is_cancelled() {
                return Err(PortError::Cancelled.into());
            }
            if let Some(job) = self.store.complete_publication(intent.assembly_id).await? {
                self.emit(JobLifecycleEventKind::Completed, &job);
                return Ok(());
            }
            if self
                .store
                .publication(intent.assembly_id)
                .await?
                .is_some_and(|publication| publication.ready)
            {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Err(conflict("Assembly files are staged; publication is waiting for recovery").into())
    }

    pub(super) async fn cleanup_unpublished(
        &self,
        staged: &[StagedArtifact],
        renderer_root: Option<std::path::PathBuf>,
    ) -> Result<(), ApplicationError> {
        let mut failure = None;
        for artifact in staged {
            if let Err(error) = self
                .artifacts
                .delete_storage_key(&artifact.staging_key)
                .await
            {
                failure.get_or_insert(error);
            }
        }
        if let Some(root) = renderer_root
            && let Some(renderer) = &self.renderer
            && let Err(error) = renderer.release_staged(root).await
        {
            failure.get_or_insert(error);
        }
        match failure {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
}
