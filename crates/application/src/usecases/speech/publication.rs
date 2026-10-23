use super::{Progress, SpeechUseCases, conflict, invalid};
use crate::error::ApplicationError;
use domain::media::ArtifactKind;
use ports::{
    PortError,
    cancellation::CancellationToken,
    job_scheduler::JobLifecycleEventKind,
    speech_run::SpeechRunIntent,
    speech_workflow::{SpeechProducedFile, SpeechPublication, SpeechPublishedFile},
};
use uuid::Uuid;

impl SpeechUseCases {
    pub(super) async fn publish_result(
        &self,
        intent: &SpeechRunIntent,
        attempt_id: Uuid,
        observer: &Progress,
        files: Vec<SpeechProducedFile>,
        duration_ms: u64,
        token: CancellationToken,
    ) -> Result<bool, ApplicationError> {
        let mut staged = Vec::new();
        let mut published = Vec::new();
        for (index, file) in files.iter().enumerate() {
            if token.is_cancelled() {
                return Err(PortError::Cancelled.into());
            }
            let kind = match file.kind.as_str() {
                "wav" => ArtifactKind::GeneratedSpeech,
                "segment" => ArtifactKind::GeneratedSpeechSegment,
                "manifest" | "audio_layout" => ArtifactKind::SpeechManifest,
                _ => return Err(invalid("Unknown speech output").into()),
            };
            let extension = if kind == ArtifactKind::SpeechManifest {
                "json"
            } else {
                "wav"
            };
            let artifact = self
                .artifacts
                .import_external_file(
                    &intent.project_id,
                    kind,
                    &file.path,
                    Some(&format!("speech-{index}.{extension}")),
                )
                .await?;
            if !self
                .artifacts
                .verify_storage_digest(&artifact.staging_key, artifact.size_bytes, &file.sha256)
                .await?
            {
                return Err(invalid("Speech staging hash mismatch").into());
            }
            published.push(SpeechPublishedFile {
                artifact_id: artifact.artifact.id.clone(),
                kind: file.kind.clone(),
                sha256: file.sha256.clone(),
                size_bytes: artifact.size_bytes,
            });
            staged.push(artifact);
        }
        if token.is_cancelled() {
            return Err(PortError::Cancelled.into());
        }
        let job = observer.job.lock().await;
        self.store
            .publish(
                &job,
                &SpeechPublication {
                    run_id: intent.run_id,
                    attempt_id,
                    duration_ms,
                    files: published,
                    ready: false,
                },
                &staged,
            )
            .await?;
        drop(job);
        for _ in 0..120 {
            if token.is_cancelled() {
                return Err(PortError::Cancelled.into());
            }
            if let Some(job) = self.store.complete_publication(intent.run_id).await? {
                self.emit(JobLifecycleEventKind::Completed, &job);
                return Ok(true);
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Err(conflict("Audio is saved; publication is waiting for recovery").into())
    }
}
