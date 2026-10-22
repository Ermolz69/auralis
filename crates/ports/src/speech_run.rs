use async_trait::async_trait;
use domain::job::Job;
use domain::project::ProjectId;
use uuid::Uuid;

use crate::error::PortError;

// Host-wide storage/transport ceilings. Admission policy can be lower, while
// immutable stored plans remain bounded independently of user configuration.
pub const MAX_SPEECH_UTTERANCES: u32 = 8192;
pub const MAX_SPEECH_CHUNKS: u32 = 8192;
pub const MAX_SPEECH_PLAN_BYTES: usize = 16_777_216;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeechRunIntent {
    pub run_id: Uuid,
    pub project_id: ProjectId,
    pub plan_json: String,
    pub plan_sha256: String,
    pub profile_id: String,
    pub profile_sha256: String,
    pub expected_utterances: u32,
    pub expected_chunks: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeechCheckpoint {
    pub run_id: Uuid,
    pub attempt_id: Uuid,
    pub plan_sha256: String,
    pub sequence: u32,
    pub utterance_id: String,
    pub cue_id: Option<String>,
    pub chunk_id: String,
    pub cache_key: String,
    pub audio_sha256: String,
    pub duration_ms: u64,
    pub origin: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveSpeechAttempt {
    pub job: Job,
    pub run_id: Uuid,
    pub attempt_id: Uuid,
}

#[async_trait]
pub trait SpeechRunStore: Send + Sync {
    async fn register_run(&self, intent: &SpeechRunIntent) -> Result<(), PortError>;
    async fn get_run(&self, run_id: Uuid) -> Result<Option<SpeechRunIntent>, PortError>;
    async fn create_pending(
        &self,
        job: Job,
        run_id: Uuid,
        attempt_id: Uuid,
    ) -> Result<Job, PortError>;
    async fn active_for_run(&self, run_id: Uuid) -> Result<Option<ActiveSpeechAttempt>, PortError>;
    async fn list_active(&self) -> Result<Vec<ActiveSpeechAttempt>, PortError>;
    async fn start(
        &self,
        job: &Job,
        attempt_id: Uuid,
        expected_revision: u64,
    ) -> Result<(), PortError>;
    async fn record_checkpoint(
        &self,
        job: &Job,
        checkpoint: &SpeechCheckpoint,
        expected_revision: u64,
    ) -> Result<(), PortError>;
    async fn finish(
        &self,
        job: &Job,
        attempt_id: Uuid,
        expected_revision: u64,
    ) -> Result<(), PortError>;
}
