use adapters_storage::memory::InMemoryProjectRepository;
use jobs::manager::JobManager;
use ports::error::PortError;
use ports::events::AppEventPublisher;
use ports::job_scheduler::JobSchedulerPort;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub mod commands;
pub mod dto;
pub mod media_tools;

#[derive(Clone)]
pub struct TauriAppEventPublisher {
    app: AppHandle,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle_clone = app.handle().clone();

            let emitter = Arc::new(move |event: ports::job_scheduler::JobLifecycleEvent| {
                // Here we might need a DTO for the frontend, but for now we emit as is if it derives Serialize
                // Oh wait, JobLifecycleEvent doesn't derive Serialize right now!
                // Wait, we need to emit it to the frontend. Wait! The event is serialized. Let's see if we should emit ScheduledJob.
                // Or maybe we can just not emit it if it fails, wait, the original emitted `jobs::event::JobEvent`.
                // For now, let's just serialize the JobLifecycleEvent, assuming we will add Serialize to it or it works.
                // Wait! Let's emit it as a JobEvent DTO, or just use the event since it's internal.
                // Actually, I should map it to a struct that can be sent to the frontend if needed, but previously we just emitted `event.clone()`.
                // Let's implement Serialize for JobLifecycleEvent later or here.

                let _ = app_handle_clone.emit(
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

                let app_clone = app_handle_clone.clone();
                tauri::async_runtime::spawn(async move {
                    use application::usecases::pipeline::handle_job_event::HandleJobEventUseCase;

                    let repo = app_clone
                        .state::<adapters_storage::memory::InMemoryProjectRepository>()
                        .inner()
                        .clone();
                    let ytdlp_adapter = crate::commands::project::get_ytdlp_adapter(&app_clone);
                    let publisher = TauriAppEventPublisher { app: app_clone };

                    let use_case = HandleJobEventUseCase::new(repo, ytdlp_adapter, publisher);
                    let _ = use_case.execute(event).await;
                });
            });

            let job_manager: Arc<dyn JobSchedulerPort> = Arc::new(JobManager::new(Some(emitter)));

            let project_repo = InMemoryProjectRepository::new();

            app.manage(job_manager);
            app.manage(project_repo);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::project::create_project_cmd,
            commands::project::create_project_from_youtube_cmd,
            commands::project::get_transcript_cmd,
            commands::project::get_project_cmd,
            commands::jobs::health_check,
            commands::jobs::start_mock_dubbing_job_cmd,
            commands::jobs::list_jobs_cmd,
            commands::jobs::cancel_job_cmd,
            commands::media::probe_local_media_cmd,
            commands::media::import_local_media_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
