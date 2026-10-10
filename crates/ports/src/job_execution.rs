use domain::job::{Job, JobId};
use domain::media::SubtitleTrack;
use domain::project::ProjectId;
use serde::{Deserialize, Serialize};

use crate::job_resources::{CpuExecutorPort, ResourceDemand};
use crate::{PortError, cancellation::CancellationToken, job_runtime_control::RuntimeTaskOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionDescriptor {
    pub operation: String,
    pub version: u32,
    pub project_id: ProjectId,
    pub selected_subtitle_track: Option<SubtitleTrack>,
    pub preferred_languages: Vec<String>,
    pub allow_auto_generated: bool,
    pub resources: ResourceDemand,
}

impl ExecutionDescriptor {
    pub const SUBTITLE_IMPORT: &'static str = "youtube_subtitle_import";

    pub fn subtitle_import(project_id: ProjectId, track: Option<SubtitleTrack>) -> Self {
        Self {
            operation: Self::SUBTITLE_IMPORT.into(),
            version: 1,
            project_id,
            selected_subtitle_track: track,
            preferred_languages: vec!["ru".into(), "en".into(), "uk".into()],
            allow_auto_generated: true,
            resources: ResourceDemand {
                cpu: 1,
                network: 1,
                disk: 1,
                processes: 1,
            },
        }
    }

    pub fn validate(&self) -> Result<(), PortError> {
        if self.operation != Self::SUBTITLE_IMPORT || self.version != 1 {
            return Err(PortError::Unsupported {
                message: "Unsupported job execution contract".into(),
            });
        }
        if self.preferred_languages.is_empty()
            || self.preferred_languages.len() > 32
            || self
                .preferred_languages
                .iter()
                .any(|v| v.trim().is_empty() || v.len() > 64)
            || self.resources == ResourceDemand::default()
            || self.selected_subtitle_track.as_ref().is_some_and(|track| {
                track.id.is_empty()
                    || track.id.len() > 4096
                    || track.language.trim().is_empty()
                    || track.language.len() > 64
                    || track.format.as_deref() != Some("vtt")
                    || track.label.as_ref().is_some_and(|label| label.len() > 4096)
            })
        {
            return Err(PortError::InvalidSource {
                message: "Invalid job execution descriptor".into(),
            });
        }
        Ok(())
    }
}

pub struct QueuedExecution {
    pub job: Job,
    pub descriptor: ExecutionDescriptor,
}

pub struct PendingExecution {
    pub job: Job,
    pub descriptor: Result<ExecutionDescriptor, PortError>,
}

pub struct ExecutionContext {
    pub cancellation: CancellationToken,
    pub cpu: std::sync::Arc<dyn CpuExecutorPort>,
    pub max_subtitle_bytes: u64,
}

#[async_trait::async_trait]
pub trait JobExecutorPort: Send + Sync {
    /// Cheap contract validation only; no preparation or external work before dispatch.
    fn validate(&self, descriptor: &ExecutionDescriptor) -> Result<(), PortError>;
    /// Must join all owned work and finish mandatory cleanup before returning.
    async fn execute(
        &self,
        execution: QueuedExecution,
        context: ExecutionContext,
    ) -> RuntimeTaskOutcome;
}

#[async_trait::async_trait]
pub trait JobExecutionStore: Send + Sync {
    async fn admit_pipeline(
        &self,
        start: crate::transaction::CommitPipelineStart,
        descriptor: ExecutionDescriptor,
        capacity: usize,
    ) -> Result<(), PortError>;
    async fn list_pending(&self, limit: usize) -> Result<Vec<PendingExecution>, PortError>;
    async fn descriptor(&self, job_id: &JobId) -> Result<Option<ExecutionDescriptor>, PortError>;
    /// Atomically checks Pending, revision, project Processing and active_job_id.
    async fn start(&self, job: &Job) -> Result<(), PortError>;
}

#[async_trait::async_trait]
pub trait JobQueuePort: Send + Sync {
    async fn admit_pipeline(
        &self,
        start: crate::transaction::CommitPipelineStart,
        descriptor: ExecutionDescriptor,
    ) -> Result<crate::job_scheduler::ScheduledJob, PortError>;
}
