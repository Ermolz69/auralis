pub mod bootstrap;
pub mod commands;
pub mod dto;
pub mod observability;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            bootstrap::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::project::create_project_cmd,
            commands::project::create_project_from_youtube_cmd,
            commands::project::get_transcript_cmd,
            commands::project::get_project_cmd,
            commands::project::list_projects_cmd,
            commands::project::delete_project_cmd,
            commands::project::start_project_mock_pipeline_cmd,
            commands::artifact::list_project_artifacts_cmd,
            commands::artifact::resolve_artifact_path_cmd,
            commands::job::health_check,
            commands::job::list_jobs_cmd,
            commands::job::cancel_job_cmd,
            commands::media::probe_local_media_cmd,
            commands::media::import_local_media_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
