use crate::bootstrap::usecases::AppUseCases;
use adapters_tauri::dto::job::JobDto;
use adapters_tauri::dto::mapper::map_job_dto;
use application::usecases::job::cancel::CancelJobRequest;
use application::usecases::job::list::ListJobsRequest;

use domain::job::JobId;
use std::str::FromStr;
use std::sync::Arc;
use tauri::{command, State};

#[command]
pub async fn health_check() -> Result<String, String> {
    Ok("ok".to_string())
}

#[command]
pub async fn list_jobs_cmd(usecases: State<'_, Arc<AppUseCases>>) -> Result<Vec<JobDto>, String> {
    let req = ListJobsRequest {};
    let res = usecases
        .list_jobs
        .execute(req)
        .await
        .map_err(|e| e.to_string())?;

    let mut dtos = Vec::with_capacity(res.jobs.len());
    for job in res.jobs {
        dtos.push(map_job_dto(&job).map_err(|e| e.to_string())?);
    }
    Ok(dtos)
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

    map_job_dto(&res.job).map_err(|e| e.to_string())
}

#[command]
pub async fn list_jobs_snapshot_cmd(
    project_id: String,
    query_port: State<'_, Arc<dyn ports::job_query::JobQueryPort>>,
) -> Result<Vec<JobDto>, String> {
    let id = domain::project::ProjectId::from_str(&project_id).map_err(|e| e.to_string())?;

    let jobs = query_port
        .list_jobs_snapshot(&id)
        .await
        .map_err(|e| e.to_string())?;

    let mut dtos = Vec::with_capacity(jobs.len());
    for job in jobs {
        dtos.push(adapters_tauri::dto::mapper::map_job_dto(&job).map_err(|e| e.to_string())?);
    }
    Ok(dtos)
}
