use async_trait::async_trait;
use domain::{job::Job, media::ArtifactId, project::ProjectId};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

use crate::{
    cancellation::CancellationToken,
    error::PortError,
    speech_run::{SpeechCheckpoint, SpeechRunIntent, SpeechRunStore},
    storage::StagedArtifact,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechSourceSnapshot {
    pub kind: String,
    pub source_id: Option<Uuid>,
    pub revision: String,
    pub sha256: String,
    pub text: String,
    pub cues: Vec<SpeechCue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechCue {
    pub id: String,
    pub text: String,
    pub role: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeechReview {
    pub decision_id: Uuid,
    pub project_id: ProjectId,
    pub source: SpeechSourceSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeechPublishedFile {
    pub artifact_id: ArtifactId,
    pub kind: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeechPublication {
    pub run_id: Uuid,
    pub attempt_id: Uuid,
    pub duration_ms: u64,
    pub files: Vec<SpeechPublishedFile>,
    pub ready: bool,
}

#[async_trait]
pub trait SpeechWorkflowStore: SpeechRunStore {
    async fn register_reviewed_run(
        &self,
        intent: &SpeechRunIntent,
        decision_id: Uuid,
    ) -> Result<(), PortError>;
    async fn save_review(&self, review: &SpeechReview) -> Result<(), PortError>;
    async fn get_review(&self, decision_id: Uuid) -> Result<Option<SpeechReview>, PortError>;
    async fn list_runs(&self, project: &ProjectId) -> Result<Vec<SpeechRunIntent>, PortError>;
    async fn latest_job(&self, run_id: Uuid) -> Result<Option<Job>, PortError>;
    async fn publication(&self, run_id: Uuid) -> Result<Option<SpeechPublication>, PortError>;
    async fn publish(
        &self,
        job: &Job,
        publication: &SpeechPublication,
        staged: &[StagedArtifact],
    ) -> Result<(), PortError>;
    async fn complete_publication(&self, run_id: Uuid) -> Result<Option<Job>, PortError>;
    async fn discard_unready_publication(
        &self,
        run_id: Uuid,
        attempt_id: Uuid,
    ) -> Result<bool, PortError>;
    async fn close_interrupted(&self) -> Result<(), PortError>;
}

pub struct SpeechProducedFile {
    pub kind: String,
    pub path: PathBuf,
    pub sha256: String,
}

pub enum SpeechExecutionResult {
    Completed {
        files: Vec<SpeechProducedFile>,
        duration_ms: u64,
    },
    Cancelled,
}

#[async_trait]
pub trait SpeechProgressObserver: Send + Sync {
    async fn checkpoint(&self, checkpoint: SpeechCheckpoint) -> Result<(), PortError>;
}

#[async_trait]
pub trait SpeechExecutionPort: Send + Sync {
    async fn compose(
        &self,
        project: ProjectId,
        run_id: Uuid,
        profile_id: String,
        draft: String,
    ) -> Result<SpeechRunIntent, PortError>;
    async fn validate(&self, intent: &SpeechRunIntent) -> Result<(), PortError>;
    async fn execute(
        &self,
        intent: SpeechRunIntent,
        attempt_id: Uuid,
        observer: Arc<dyn SpeechProgressObserver>,
        cancel: CancellationToken,
    ) -> Result<SpeechExecutionResult, PortError>;
}
