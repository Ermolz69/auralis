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
                if event_status == jobs::status::JobStatus::Completed
                    || event_status == jobs::status::JobStatus::Failed
                {
                    if let Some(project_id_str) = &event.project_id {
                        let app_clone = app_handle_clone.clone();
                        let pid_str = project_id_str.clone();
                        let is_success = event_status == jobs::status::JobStatus::Completed;

                        tauri::async_runtime::spawn(async move {
                            use ports::repository::ProjectRepository;
                            use std::str::FromStr;

                            let repo = app_clone
                                .state::<adapters_storage::memory::InMemoryProjectRepository>();
                            if let Ok(pid) = domain::project::ProjectId::from_str(&pid_str) {
                                if let Ok(Some(mut project)) = repo.get(&pid).await {
                                    if is_success {
                                        let is_youtube = matches!(
                                            project.source(),
                                            Some(domain::media::MediaSource::YoutubeUrl { .. })
                                        );

                                        if is_youtube {
                                            // Fetch subtitles first
                                            use application::usecases::transcript::import_youtube_subtitles::{
                                                ImportYoutubeSubtitlesRequest,
                                                ImportYoutubeSubtitlesUseCase,
                                            };
                                            let ytdlp_adapter =
                                                crate::commands::project::get_ytdlp_adapter(&app_clone);
                                            let target_dir = std::env::temp_dir()
                                                .join("auralis")
                                                .join("projects")
                                                .join(&pid_str)
                                                .join("subtitles");
                                            let use_case = ImportYoutubeSubtitlesUseCase::new(
                                                std::sync::Arc::new(repo.inner().clone()),
                                                std::sync::Arc::new(ytdlp_adapter),
                                            );
                                            let _ = use_case
                                                .execute(ImportYoutubeSubtitlesRequest {
                                                    project_id: pid.clone(),
                                                    target_dir,
                                                    preferred_languages: vec![
                                                        "en".to_string(),
                                                        "ru".to_string(),
                                                        "uk".to_string(),
                                                    ],
                                                    allow_auto_generated: true,
                                                })
                                                .await;
                                            // Re-fetch project to ensure we have the updated version with transcript
                                            if let Ok(Some(updated_project)) = repo.get(&pid).await {
                                                project = updated_project;
                                            }
                                        }
                                        let _ = project.mark_completed();
                                    } else {
                                        let _ = project.mark_failed();
                                    }
                                    let _ = repo.save(&project).await;
                                    if is_success {
                                        let _ = app_clone.emit(
                                            "transcript-ready",
                                            serde_json::json!({
                                                "projectId": pid_str,
                                                "jobId": event_job_id,
                                            }),
                                        );
                                    }
                                }
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
