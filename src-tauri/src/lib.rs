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
            let app_handle = app.handle().clone();

            let app_handle_clone = app_handle.clone();
            let emitter = Arc::new(move |event: jobs::event::JobEvent| {
                if event.status == jobs::status::JobStatus::Completed {
                    if let Some(project_id) = &event.project_id {
                        let _ = app_handle_clone.emit(
                            "transcript-ready",
                            serde_json::json!({
                                "projectId": project_id,
                                "jobId": event.job_id,
                            }),
                        );
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
            commands::project::run_dubbing_cmd,
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
