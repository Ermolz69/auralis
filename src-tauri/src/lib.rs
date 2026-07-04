#[tauri::command]
fn run_dubbing_cmd(video_url: String) -> Result<String, String> {
    application::commands::run_dubbing(video_url)
}

#[tauri::command]
async fn health_check() -> Result<String, String> {
    Ok("ok".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_dubbing_cmd, health_check])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
