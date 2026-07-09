use domain::job::JobId;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};
use std::str::FromStr;
use std::sync::Arc;
use tauri::{command, State};

#[command]
pub async fn health_check() -> Result<String, String> {
    Ok("ok".to_string())
}

#[command]
pub async fn start_mock_dubbing_job_cmd(
    input: String,
    state: State<'_, Arc<dyn JobSchedulerPort>>,
) -> Result<ScheduledJob, String> {
    state
        .start_dubbing_job(StartDubbingJobRequest {
            title: input,
            project_id: None,
        })
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn list_jobs_cmd(
    state: State<'_, Arc<dyn JobSchedulerPort>>,
) -> Result<Vec<ScheduledJob>, String> {
    state.list_jobs().await.map_err(|e| e.to_string())
}

#[command]
pub async fn cancel_job_cmd(
    job_id: String,
    state: State<'_, Arc<dyn JobSchedulerPort>>,
) -> Result<ScheduledJob, String> {
    let id = JobId::from_str(&job_id).map_err(|e| e.to_string())?;
    state.cancel_job(&id).await.map_err(|e| e.to_string())
}
