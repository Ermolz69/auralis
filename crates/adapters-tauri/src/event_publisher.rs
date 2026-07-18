use crate::dto::job_event::{ProjectUpdatedDto, TranscriptReadyDto};
use async_trait::async_trait;
use ports::error::PortError;
use ports::events::AppEventPublisher;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct TauriEventPublisher {
    app: AppHandle,
}

pub trait FrontendJobEventPublisher: Send + Sync {
    fn publish_job_event(
        &self,
        event: &ports::job_scheduler::JobLifecycleEvent,
    ) -> Result<(), PortError>;

    fn publish_invalidated(&self) -> Result<(), PortError>;
}

impl TauriEventPublisher {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

pub const EVENT_JOB_EVENT: &str = "job-event";
pub const EVENT_JOB_EVENTS_INVALIDATED: &str = "job-events-invalidated";

impl FrontendJobEventPublisher for TauriEventPublisher {
    fn publish_job_event(
        &self,
        event: &ports::job_scheduler::JobLifecycleEvent,
    ) -> Result<(), PortError> {
        let dto =
            crate::dto::mapper::map_job_event_dto(event).map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;
        self.app
            .emit(EVENT_JOB_EVENT, dto)
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })
    }

    fn publish_invalidated(&self) -> Result<(), PortError> {
        self.app
            .emit(EVENT_JOB_EVENTS_INVALIDATED, ())
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })
    }
}

#[async_trait]
impl AppEventPublisher for TauriEventPublisher {
    async fn publish_project_updated(&self, project_id: &str) -> Result<(), PortError> {
        self.app
            .emit(
                "project-updated",
                ProjectUpdatedDto {
                    project_id: project_id.to_string(),
                },
            )
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })
    }

    async fn publish_transcript_ready(
        &self,
        project_id: &str,
        job_id: &str,
    ) -> Result<(), PortError> {
        self.app
            .emit(
                "transcript-ready",
                TranscriptReadyDto {
                    project_id: project_id.to_string(),
                    job_id: job_id.to_string(),
                },
            )
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })
    }
}
