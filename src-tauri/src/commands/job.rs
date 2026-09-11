use crate::bootstrap::usecases::AppUseCases;
use crate::dto::error::{CommandError, map_job_dto_result, parse_job_id, parse_project_id};
use adapters_tauri::dto::job::JobDto;
use adapters_tauri::dto::mapper::map_job_dto;
use application::usecases::job::cancel::CancelJobRequest;
use application::usecases::job::list::ListJobsRequest;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{State, command};

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn health_check(request: tauri::ipc::Request<'_>) -> Result<String, CommandError> {
    crate::observability::command::observe("health_check", async { Ok("ok".to_string()) }).await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_jobs_cmd(
    request: tauri::ipc::Request<'_>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<JobDto>, CommandError> {
    crate::observability::command::observe("list_jobs_cmd", async {
        let req = ListJobsRequest {};
        let res = usecases
            .list_jobs
            .execute(req)
            .await
            .map_err(CommandError::from)?;

        let mut dtos = Vec::with_capacity(res.jobs.len());
        for job in res.jobs {
            dtos.push(map_job_dto_result(map_job_dto(&job))?);
        }
        Ok(dtos)
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn cancel_job_cmd(
    request: tauri::ipc::Request<'_>,
    job_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<JobDto, CommandError> {
    crate::observability::command::observe("cancel_job_cmd", async {
        let id = parse_job_id(&job_id)?;

        let req = CancelJobRequest { job_id: id };
        let res = usecases
            .cancel_job
            .execute(req)
            .await
            .map_err(CommandError::from)?;

        map_job_dto_result(map_job_dto(&res.job))
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_jobs_snapshot_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    query_port: State<'_, Arc<dyn ports::job_query::JobQueryPort>>,
) -> Result<Vec<JobDto>, CommandError> {
    crate::observability::command::observe("list_jobs_snapshot_cmd", async {
        let id = parse_project_id(&project_id)?;

        let jobs = query_port
            .list_jobs_snapshot(&id)
            .await
            .map_err(CommandError::from)?;

        let mut dtos = Vec::with_capacity(jobs.len());
        for job in jobs {
            dtos.push(map_job_dto_result(
                adapters_tauri::dto::mapper::map_job_dto(&job),
            )?);
        }
        Ok(dtos)
    })
    .await
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryCursorDto {
    created_at: String,
    job_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryPageDto {
    jobs: Vec<JobDto>,
    next_cursor: Option<JobHistoryCursorDto>,
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_job_history_page_cmd(
    request: tauri::ipc::Request<'_>,
    cursor: Option<JobHistoryCursorDto>,
    limit: Option<u32>,
    query_port: State<'_, Arc<dyn ports::job_query::JobQueryPort>>,
) -> Result<JobHistoryPageDto, CommandError> {
    crate::observability::command::observe("list_job_history_page_cmd", async {
        let limit = parse_history_page_size(limit)?;
        let cursor = cursor.map(parse_history_cursor).transpose()?;
        let page = query_port
            .list_job_history_page(cursor.as_ref(), limit)
            .await
            .map_err(CommandError::from)?;

        let mut jobs = Vec::with_capacity(page.jobs.len());
        for job in page.jobs {
            jobs.push(map_job_dto_result(map_job_dto(&job))?);
        }

        Ok(JobHistoryPageDto {
            jobs,
            next_cursor: page.next_cursor.map(JobHistoryCursorDto::from),
        })
    })
    .await
}

fn parse_history_page_size(limit: Option<u32>) -> Result<usize, CommandError> {
    let limit = usize::try_from(limit.unwrap_or(100))
        .map_err(|_| CommandError::Validation("Invalid history page size".to_string()))?;
    if !(1..=ports::job_query::MAX_JOB_HISTORY_PAGE_SIZE).contains(&limit) {
        return Err(CommandError::Validation(
            "History page size must be between 1 and 100".to_string(),
        ));
    }
    Ok(limit)
}

fn parse_history_cursor(
    cursor: JobHistoryCursorDto,
) -> Result<ports::job_query::JobHistoryCursor, CommandError> {
    let created_at = chrono::DateTime::parse_from_rfc3339(&cursor.created_at)
        .map_err(|_| CommandError::Validation("Invalid history cursor".to_string()))?
        .with_timezone(&chrono::Utc);
    let job_id = parse_job_id(&cursor.job_id)?;
    Ok(ports::job_query::JobHistoryCursor { created_at, job_id })
}

impl From<ports::job_query::JobHistoryCursor> for JobHistoryCursorDto {
    fn from(cursor: ports::job_query::JobHistoryCursor) -> Self {
        Self {
            created_at: cursor.created_at.to_rfc3339(),
            job_id: cursor.job_id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_page_size_is_bounded() {
        assert_eq!(parse_history_page_size(None).ok(), Some(100));
        assert_eq!(parse_history_page_size(Some(1)).ok(), Some(1));
        assert_eq!(parse_history_page_size(Some(100)).ok(), Some(100));
        assert!(matches!(
            parse_history_page_size(Some(0)),
            Err(CommandError::Validation(_))
        ));
        assert!(matches!(
            parse_history_page_size(Some(101)),
            Err(CommandError::Validation(_))
        ));
    }

    #[test]
    fn history_cursor_parses_and_rejects_invalid_values() {
        let job_id = domain::job::JobId::new();
        let parsed = parse_history_cursor(JobHistoryCursorDto {
            created_at: "2026-09-07T12:00:00Z".to_string(),
            job_id: job_id.to_string(),
        });
        assert!(matches!(parsed, Ok(cursor) if cursor.job_id == job_id));

        assert!(matches!(
            parse_history_cursor(JobHistoryCursorDto {
                created_at: "not-a-timestamp".to_string(),
                job_id: job_id.to_string(),
            }),
            Err(CommandError::Validation(_))
        ));
        assert!(matches!(
            parse_history_cursor(JobHistoryCursorDto {
                created_at: "2026-09-07T12:00:00Z".to_string(),
                job_id: "not-a-job-id".to_string(),
            }),
            Err(CommandError::Validation(_))
        ));
    }
}
