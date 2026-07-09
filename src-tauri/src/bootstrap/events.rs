use ports::error::PortError;
use ports::events::AppEventPublisher;
use ports::job_scheduler::JobLifecycleEvent;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone)]
pub struct TauriAppEventPublisher {
    app: AppHandle,
}

impl TauriAppEventPublisher {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl AppEventPublisher for TauriAppEventPublisher {
    async fn publish_project_updated(&self, project_id: &str) -> Result<(), PortError> {
        self.app
            .emit(
                "project-updated",
                serde_json::json!({
                    "projectId": project_id,
                }),
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
                serde_json::json!({
                    "projectId": project_id,
                    "jobId": job_id,
                }),
            )
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })
    }
}

pub fn build_job_event_emitter(
    app_handle: AppHandle,
) -> Arc<dyn Fn(JobLifecycleEvent) + Send + Sync> {
    Arc::new(move |event: JobLifecycleEvent| {
        let _ = app_handle.emit(
            "job-event",
            serde_json::json!({
                "jobId": event.job_id.to_string(),
                "projectId": event.project_id.as_ref().map(|id| id.to_string()),
                "status": event.status,
                "stage": event.stage,
                "progress": event.progress,
                "error": event.error,
            }),
        );

        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            use application::usecases::pipeline::handle_job_event::HandleJobEventUseCase;

            let repo = app_clone
                .state::<crate::state::RuntimeProjectRepository>()
                .inner()
                .clone();
            let publisher = TauriAppEventPublisher::new(app_clone);

            let use_case = HandleJobEventUseCase::new(repo, publisher);
            let _ = use_case.execute(event).await;
        });
    })
}
