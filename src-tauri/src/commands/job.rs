use crate::bootstrap::usecases::AppUseCases;
use crate::dto::job::JobDto;
use application::usecases::job::cancel::CancelJobRequest;
use application::usecases::job::list::ListJobsRequest;
use application::usecases::job::start_mock::StartMockJobRequest;
use domain::job::JobId;
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
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<JobDto, String> {
    let req = StartMockJobRequest { title: input };
    let res = usecases
        .start_mock_job
        .execute(req)
        .await
        .map_err(|e| e.to_string())?;

    Ok(JobDto::from(&res.job))
}

#[command]
pub async fn list_jobs_cmd(usecases: State<'_, Arc<AppUseCases>>) -> Result<Vec<JobDto>, String> {
    let req = ListJobsRequest {};
    let res = usecases
        .list_jobs
        .execute(req)
        .await
        .map_err(|e| e.to_string())?;

    Ok(res.jobs.into_iter().map(|job| JobDto::from(&job)).collect())
}

#[command]
pub async fn cancel_job_cmd(
    job_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<JobDto, String> {
    let id = JobId::from_str(&job_id).map_err(|e| e.to_string())?;

    let req = CancelJobRequest { job_id: id };
    let res = usecases
        .cancel_job
        .execute(req)
        .await
        .map_err(|e| e.to_string())?;

    Ok(JobDto::from(&res.job))
}
