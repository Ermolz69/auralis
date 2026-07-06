use jobs::id::JobId;
use jobs::job::Job;
use jobs::manager::JobManager;
use tauri::{command, State};

#[command]
pub async fn health_check() -> Result<String, String> {
    Ok("ok".to_string())
}

#[command]
pub async fn start_mock_dubbing_job_cmd(
    input: String,
    state: State<'_, JobManager>,
) -> Result<Job, String> {
    let job_id = state.start_mock_dubbing_job(input, None).await;
    state
        .get_job(&job_id)
        .await
        .ok_or_else(|| "Failed to retrieve job".to_string())
}

#[command]
pub async fn list_jobs_cmd(state: State<'_, JobManager>) -> Result<Vec<Job>, String> {
    Ok(state.list_jobs().await)
}

#[command]
pub async fn cancel_job_cmd(job_id: String, state: State<'_, JobManager>) -> Result<Job, String> {
    let id = JobId(job_id);
    state.cancel_job(&id).await.map_err(|e| e.to_string())
}
