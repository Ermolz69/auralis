use super::*;
use crate::error::ApplicationError;
use domain::{
    media::{ArtifactLocation, ArtifactState},
    project::ProjectId,
};
use ports::{
    job_scheduler::JobLifecycleEvent, job_scheduler::ScheduledJob, speech_run::SpeechRunIntent,
    speech_workflow::SpeechPublication,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct SpeechRunView {
    pub technical_reference_permission: Option<String>,
    pub run_id: Uuid,
    pub profile_id: String,
    pub utterance_count: u32,
    pub chunk_count: u32,
    pub job: Option<ScheduledJob>,
    pub result: Option<SpeechPublication>,
}

impl SpeechUseCases {
    pub async fn list(&self, project: ProjectId) -> Result<Vec<SpeechRunView>, ApplicationError> {
        self.projects
            .get(&project)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project.clone()))?;
        let mut results = Vec::new();
        for id in self.store.list_run_ids(&project).await? {
            if let Some(intent) = self.store.get_run(id).await? {
                results.push(self.view(intent).await?);
            }
        }
        Ok(results)
    }

    pub async fn state(
        &self,
        project: ProjectId,
        run_id: Uuid,
    ) -> Result<SpeechRunView, ApplicationError> {
        let intent = self.intent(&project, run_id).await?;
        self.view(intent).await
    }

    pub(super) async fn intent(
        &self,
        project: &ProjectId,
        run_id: Uuid,
    ) -> Result<SpeechRunIntent, ApplicationError> {
        let intent = self
            .store
            .get_run(run_id)
            .await?
            .ok_or_else(|| invalid("Speech run not found"))?;
        if &intent.project_id != project {
            return Err(conflict("Speech run belongs to another project").into());
        }
        Ok(intent)
    }

    pub(super) async fn view(
        &self,
        intent: SpeechRunIntent,
    ) -> Result<SpeechRunView, ApplicationError> {
        if let Some(publication) = self.store.publication(intent.run_id).await?
            && !publication.ready
            && self
                .store
                .latest_job(intent.run_id)
                .await?
                .is_some_and(|j| j.status() == &domain::job::JobStatus::Cancelled)
        {
            self.store
                .discard_unready_publication(intent.run_id, publication.attempt_id)
                .await?;
        }
        self.store.close_interrupted().await?;
        if let Some(job) = self.store.complete_publication(intent.run_id).await? {
            self.emit(ports::job_scheduler::JobLifecycleEventKind::Completed, &job);
        }
        let result = self
            .store
            .publication(intent.run_id)
            .await?
            .filter(|r| r.ready);
        if let Some(ref publication) = result {
            for file in &publication.files {
                let artifact = self
                    .index
                    .get(&file.artifact_id)
                    .await?
                    .ok_or_else(|| invalid("Published speech artifact is missing"))?;
                let ArtifactLocation::StorageKey(key) = &artifact.location else {
                    return Err(invalid("Speech artifact is not managed").into());
                };
                if artifact.state != ArtifactState::Ready
                    || !self
                        .artifacts
                        .verify_storage_digest(key, file.size_bytes, &file.sha256)
                        .await?
                {
                    return Err(invalid("Published speech artifact failed its hash check").into());
                }
            }
        }
        let job = self
            .store
            .latest_job(intent.run_id)
            .await?
            .map(|job| ScheduledJob::from(job.to_snapshot()));
        let plan: serde_json::Value = serde_json::from_str(&intent.plan_json)
            .map_err(|error| invalid(format!("Saved speech plan is invalid: {error}")))?;
        Ok(SpeechRunView {
            technical_reference_permission: plan["review"]["technical_reference_permission"]
                .as_str()
                .map(str::to_owned),
            run_id: intent.run_id,
            profile_id: intent.profile_id,
            utterance_count: intent.expected_utterances,
            chunk_count: intent.expected_chunks,
            job,
            result,
        })
    }

    pub(super) fn emit(
        &self,
        kind: ports::job_scheduler::JobLifecycleEventKind,
        job: &domain::job::Job,
    ) {
        (self.emitter)(JobLifecycleEvent {
            kind,
            job: ScheduledJob::from(job.to_snapshot()),
        });
    }
}
