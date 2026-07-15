use domain::job::{Job, JobId, JobSnapshot};
use domain::project::ProjectId;
use ports::error::PortError;
use std::str::FromStr;

use super::job_row::JobRow;

pub fn row_to_job(row: JobRow) -> Result<Job, PortError> {
    let id = JobId::from_str(&row.id).map_err(|e| PortError::Unexpected {
        message: format!("Failed to parse job id `{}`: {}", row.id, e),
    })?;

    let project_id = ProjectId::from_str(&row.project_id).map_err(|e| PortError::Unexpected {
        message: format!("Failed to parse project id `{}`: {}", row.project_id, e),
    })?;

    let kind = parse_json(&format!("\"{}\"", row.kind), "kind")?;
    let status = parse_json(&format!("\"{}\"", row.status), "status")?;

    let stage = row
        .stage
        .map(|s| parse_json(&format!("\"{}\"", s), "stage"))
        .transpose()?;

    let progress = parse_json(&row.progress_json, "progress_json")?;
    let error = row
        .error_json
        .map(|s| parse_json(&s, "error_json"))
        .transpose()?;

    let created_at = parse_datetime(&row.created_at, "created_at")?;
    let started_at = row
        .started_at
        .map(|s| parse_datetime(&s, "started_at"))
        .transpose()?;
    let finished_at = row
        .finished_at
        .map(|s| parse_datetime(&s, "finished_at"))
        .transpose()?;

    let snapshot = JobSnapshot {
        id,
        project_id,
        title: row.title,
        kind,
        status,
        stage,
        progress,
        error,
        created_at,
        updated_at: parse_datetime(&row.updated_at, "updated_at")?,
        started_at,
        finished_at,
    };

    Ok(Job::from_snapshot(snapshot))
}

pub fn job_to_row_values(job: &Job) -> Result<JobRow, PortError> {
    use super::helpers::{serialize_enum, serialize_json};

    let snapshot = job.to_snapshot();

    let kind = serialize_enum(&snapshot.kind, "job.kind")?;
    let status = serialize_enum(&snapshot.status, "job.status")?;
    let stage = snapshot
        .stage
        .map(|s| serialize_enum(&s, "job.stage"))
        .transpose()?;

    let progress_json = serialize_json(&snapshot.progress, "job.progress")?;
    let error_json = snapshot
        .error
        .map(|e| serialize_json(&e, "job.error"))
        .transpose()?;

    Ok(JobRow {
        id: snapshot.id.to_string(),
        project_id: snapshot.project_id.to_string(),
        title: snapshot.title,
        kind,
        status,
        stage,
        progress_json,
        error_json,
        created_at: snapshot.created_at.to_rfc3339(),
        updated_at: snapshot.updated_at.to_rfc3339(),
        started_at: snapshot.started_at.map(|d| d.to_rfc3339()),
        finished_at: snapshot.finished_at.map(|d| d.to_rfc3339()),
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(value: &str, field: &str) -> Result<T, PortError> {
    super::helpers::deserialize_json(value, field)
}

fn parse_datetime(value: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, PortError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to parse field `{}` as datetime: {}", field, e),
        })
}
