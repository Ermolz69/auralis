use super::{
    AudioAssemblyUseCases,
    error::{conflict, invalid},
};
use crate::error::ApplicationError;
use domain::{
    job::JobStatus,
    media::{ArtifactKind, ArtifactLocation, ArtifactState},
    project::ProjectId,
    speech::Sha256Digest,
};
use ports::{
    audio_inspection::SpeechTimingSettings,
    audio_jobs::{AudioAssemblyIntent, AudioAssemblyPlan, AudioAssemblyPublication},
    job_scheduler::{JobLifecycleEvent, JobLifecycleEventKind, ScheduledJob},
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct AudioAssemblyView {
    pub video: Option<ports::video_export::VideoAssemblySource>,
    pub assembly_id: Uuid,
    pub source_run_id: Uuid,
    pub plan_sha256: Sha256Digest,
    pub settings: SpeechTimingSettings,
    pub utterance_count: usize,
    pub frames: u64,
    pub job: Option<ScheduledJob>,
    pub result: Option<AudioAssemblyPublication>,
}

impl AudioAssemblyUseCases {
    pub async fn verify(
        &self,
        project: ProjectId,
        id: Uuid,
    ) -> Result<AudioAssemblyView, ApplicationError> {
        let state = self.state(project, id).await?;
        let publication = state
            .result
            .as_ref()
            .ok_or_else(|| invalid("Wait for complete assembly publication before playback"))?;
        tokio::time::timeout(std::time::Duration::from_secs(120), async {
            for file in &publication.files {
                let artifact = self
                    .index
                    .get(&file.artifact_id)
                    .await?
                    .ok_or_else(|| invalid("Published assembly artifact is missing"))?;
                let ArtifactLocation::StorageKey(key) = &artifact.location else {
                    return Err(invalid("Published assembly artifact is not managed"));
                };
                if !self
                    .artifacts
                    .verify_storage_digest(key, file.size_bytes, &file.sha256)
                    .await?
                {
                    return Err(invalid("Published assembly artifact failed its hash check"));
                }
            }
            Ok::<_, ports::PortError>(())
        })
        .await
        .map_err(|_| invalid("Assembly playback verification exceeded its deadline"))??;
        Ok(state)
    }
    pub async fn state(
        &self,
        project: ProjectId,
        id: Uuid,
    ) -> Result<AudioAssemblyView, ApplicationError> {
        self.view(self.intent(&project, id).await?).await
    }
    pub async fn list(
        &self,
        project: ProjectId,
    ) -> Result<Vec<AudioAssemblyView>, ApplicationError> {
        self.projects
            .get(&project)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project.clone()))?;
        let mut results = Vec::new();
        for id in self.store.list_ids(&project).await? {
            if let Some(intent) = self.store.get(id).await? {
                results.push(self.view(intent).await?);
            }
        }
        Ok(results)
    }
    pub(super) async fn intent(
        &self,
        project: &ProjectId,
        id: Uuid,
    ) -> Result<AudioAssemblyIntent, ApplicationError> {
        let intent = self
            .store
            .get(id)
            .await?
            .ok_or_else(|| invalid("Audio assembly intent is missing"))?;
        if &intent.project_id != project {
            return Err(conflict("Audio assembly belongs to another project").into());
        }
        Ok(intent)
    }
    pub(super) async fn view(
        &self,
        intent: AudioAssemblyIntent,
    ) -> Result<AudioAssemblyView, ApplicationError> {
        let plan: AudioAssemblyPlan = serde_json::from_str(&intent.plan_json)
            .map_err(|_| invalid("Stored assembly plan is invalid"))?;
        let timeline = domain::speech::timeline::plan_audio_timeline(&plan.timeline)
            .map_err(|_| invalid("Stored assembly timing conflicts"))?;
        if let Some(publication) = self.store.publication(intent.assembly_id).await?
            && !publication.ready
            && self
                .store
                .latest_job(intent.assembly_id)
                .await?
                .is_some_and(|job| job.status() == &JobStatus::Cancelled)
        {
            self.store
                .discard_unready(intent.assembly_id, publication.attempt_id)
                .await?;
        }
        self.store.close_interrupted().await?;
        if let Some(job) = self.store.complete_publication(intent.assembly_id).await? {
            self.emit(JobLifecycleEventKind::Completed, &job);
        }
        let result = self
            .store
            .publication(intent.assembly_id)
            .await?
            .filter(|publication| publication.ready);
        if let Some(publication) = &result {
            for file in &publication.files {
                let artifact = self
                    .index
                    .get(&file.artifact_id)
                    .await?
                    .ok_or_else(|| invalid("Published assembly artifact is missing"))?;
                let ArtifactLocation::StorageKey(key) = &artifact.location else {
                    return Err(invalid("Published assembly artifact is not managed").into());
                };
                if !key.starts_with(&format!("{}/", intent.project_id))
                    || artifact.state != ArtifactState::Ready
                    || artifact.size_bytes != Some(file.size_bytes)
                    || artifact.kind
                        != match file.kind.as_str() {
                            "wav" => ArtifactKind::MixedAudio,
                            "video" => ArtifactKind::FinalVideo,
                            "manifest" | "video_manifest" => ArtifactKind::SpeechManifest,
                            _ => return Err(invalid("Unknown published assembly role").into()),
                        }
                {
                    return Err(invalid("Published assembly artifact failed its hash check").into());
                }
            }
        }
        Ok(AudioAssemblyView {
            video: plan.video,
            assembly_id: intent.assembly_id,
            source_run_id: intent.source_run_id,
            plan_sha256: intent.plan_sha256,
            settings: SpeechTimingSettings {
                mode: plan.timeline.mode,
                source_duration_ms: plan.timeline.source_duration_ms,
                pause_after_ms: plan
                    .timeline
                    .utterances
                    .iter()
                    .map(|utterance| utterance.pause_after_ms)
                    .collect(),
            },
            utterance_count: plan.timeline.utterances.len(),
            frames: timeline.frame_count,
            job: self
                .store
                .latest_job(intent.assembly_id)
                .await?
                .map(|job| ScheduledJob::from(job.to_snapshot())),
            result,
        })
    }
    pub(super) fn emit(&self, kind: JobLifecycleEventKind, job: &domain::job::Job) {
        (self.emitter)(JobLifecycleEvent {
            kind,
            job: ScheduledJob::from(job.to_snapshot()),
        });
    }
}
