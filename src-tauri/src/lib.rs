use adapters_storage::memory::InMemoryProjectRepository;
use jobs::manager::JobManager;
use std::sync::Arc;
use tauri::{Emitter, Manager};

pub mod commands;
pub mod dto;
pub mod media_tools;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle_clone = app.handle().clone();
            let emitter = Arc::new(move |event: jobs::event::JobEvent| {
                let event_status = event.status;
                let event_job_id = event.job_id.clone();

                if matches!(
                    event_status,
                    jobs::status::JobStatus::Completed
                        | jobs::status::JobStatus::Failed
                        | jobs::status::JobStatus::Cancelled
                ) {
                    if let Some(project_id_str) = &event.project_id {
                        let app_clone = app_handle_clone.clone();
                        let pid_str = project_id_str.clone();

                        tauri::async_runtime::spawn(async move {
                            let project_update_result = match event_status {
                                jobs::status::JobStatus::Completed
                                | jobs::status::JobStatus::Failed => {
                                    use application::usecases::project::handle_job_completed::{
                                        HandleJobCompletedRequest, HandleJobCompletedUseCase,
                                    };

                                    let repo = app_clone
                                        .state::<InMemoryProjectRepository>()
                                        .inner()
                                        .clone();
                                    let ytdlp_adapter =
                                        crate::commands::project::get_ytdlp_adapter(&app_clone);
                                    let use_case = HandleJobCompletedUseCase::new(repo, ytdlp_adapter);
                                    let is_success =
                                        event_status == jobs::status::JobStatus::Completed;

                                    use_case
                                        .execute(HandleJobCompletedRequest {
                                            job_id: event_job_id.to_string(),
                                            project_id: pid_str.clone(),
                                            is_success,
                                            target_dir_base: std::env::temp_dir(),
                                        })
                                        .await
                                        .map(|result| result.transcript_ready)
                                }
                                jobs::status::JobStatus::Cancelled => {
                                    use application::usecases::project::handle_job_cancelled::{
                                        HandleJobCancelledRequest, HandleJobCancelledUseCase,
                                    };

                                    let repo = app_clone
                                        .state::<InMemoryProjectRepository>()
                                        .inner()
                                        .clone();
                                    let use_case = HandleJobCancelledUseCase::new(repo);

                                    use_case
                                        .execute(HandleJobCancelledRequest {
                                            job_id: event_job_id.to_string(),
                                            project_id: pid_str.clone(),
                                        })
                                        .await
                                        .map(|_| false)
                                        .map_err(|error| error.to_string())
                                }
                                _ => Ok(false),
                            };

                            if let Ok(transcript_ready) = project_update_result {
                                if transcript_ready {
                                    let _ = app_clone.emit(
                                        "transcript-ready",
                                        serde_json::json!({
                                            "projectId": pid_str,
                                            "jobId": event_job_id,
                                        }),
                                    );
                                }

                                let _ = app_clone.emit(
                                    "project-updated",
                                    serde_json::json!({
                                        "projectId": pid_str,
                                    }),
                                );
                            }
                        });
                    }
                }

                let _ = app_handle_clone.emit("job-event", event);
            });
            let job_manager = JobManager::new(Some(emitter));

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
