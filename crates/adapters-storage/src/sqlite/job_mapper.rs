use domain::job::{Job, JobId, JobSnapshot};
use domain::project::ProjectId;
use ports::error::PortError;
use std::str::FromStr;

use super::job_row::JobRow;

pub fn row_to_job(row: JobRow) -> Result<Job, PortError> {
    let id = parse_job_id(&row.id)?;

    let revision = u64::try_from(row.revision).map_err(|e| PortError::InvalidStoredData {
        entity_type: "job".to_string(),
        entity_id: row.id.clone(),
        field: "revision".to_string(),
        message: format!("Failed to parse revision `{}`: {}", row.revision, e),
    })?;

    if revision == 0 || revision > domain::job::MAX_JOB_REVISION {
        return Err(PortError::InvalidStoredData {
            entity_type: "job".to_string(),
            entity_id: row.id.clone(),
            field: "revision".to_string(),
            message: format!("Revision {} is out of bounds", revision),
        });
    }

    let project_id = parse_project_id(&row.project_id, &row.id)?;

    let kind = parse_json_field(&format!("\"{}\"", row.kind), &row.id, "kind")?;
    let status = parse_json_field(&format!("\"{}\"", row.status), &row.id, "status")?;

    let stage = row
        .stage
        .map(|s| parse_json_field(&format!("\"{}\"", s), &row.id, "stage"))
        .transpose()?;

    let progress = parse_json_field(&row.progress_json, &row.id, "progress_json")?;
    let error = row
        .error_json
        .map(|s| parse_json_field(&s, &row.id, "error_json"))
        .transpose()?;

    let created_at = parse_datetime(&row.created_at, &row.id, "created_at")?;
    let started_at = row
        .started_at
        .map(|s| parse_datetime(&s, &row.id, "started_at"))
        .transpose()?;
    let finished_at = row
        .finished_at
        .map(|s| parse_datetime(&s, &row.id, "finished_at"))
        .transpose()?;

    let snapshot = JobSnapshot {
        id,
        revision,
        project_id,
        title: row.title,
        kind,
        status,
        stage,
        progress,
        error,
        created_at,
        updated_at: parse_datetime(&row.updated_at, &row.id, "updated_at")?,
        started_at,
        finished_at,
    };

    Ok(Job::from_snapshot(snapshot))
}

pub fn job_to_row_values(job: &Job) -> Result<JobRow, PortError> {
    use super::helpers::{serialize_enum, serialize_json};

    let snapshot = job.to_snapshot();

    let revision = i64::try_from(snapshot.revision).map_err(|e| PortError::Storage {
        operation: "job_to_row_values",
        message: format!("Failed to convert revision `{}`: {}", snapshot.revision, e),
    })?;

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
        revision,
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

fn parse_job_id(value: &str) -> Result<JobId, PortError> {
    JobId::from_str(value).map_err(|e| PortError::InvalidStoredData {
        entity_type: "job".to_string(),
        entity_id: value.to_string(),
        field: "id".to_string(),
        message: format!("Failed to parse job id: {}", e),
    })
}

fn parse_project_id(value: &str, job_id: &str) -> Result<ProjectId, PortError> {
    ProjectId::from_str(value).map_err(|e| PortError::InvalidStoredData {
        entity_type: "job".to_string(),
        entity_id: job_id.to_string(),
        field: "project_id".to_string(),
        message: format!("Failed to parse project id `{}`: {}", value, e),
    })
}

fn parse_json_field<T: serde::de::DeserializeOwned>(
    value: &str,
    job_id: &str,
    field: &'static str,
) -> Result<T, PortError> {
    serde_json::from_str(value).map_err(|e| PortError::InvalidStoredData {
        entity_type: "job".to_string(),
        entity_id: job_id.to_string(),
        field: field.to_string(),
        message: format!("Failed to deserialize: {}", e),
    })
}

fn parse_datetime(
    value: &str,
    job_id: &str,
    field: &'static str,
) -> Result<chrono::DateTime<chrono::Utc>, PortError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| PortError::InvalidStoredData {
            entity_type: "job".to_string(),
            entity_id: job_id.to_string(),
            field: field.to_string(),
            message: format!("Failed to parse datetime: {}", e),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_job_row() -> JobRow {
        JobRow {
            id: "00000000-0000-0000-0000-000000000001".to_string(),
            revision: 1,
            project_id: "00000000-0000-0000-0000-000000000002".to_string(),
            title: "Test Job".to_string(),
            kind: "dubbing".to_string(),
            status: "pending".to_string(),
            stage: None,
            progress_json: "{\"percent\": 50, \"message\": \"halfway\"}".to_string(),
            error_json: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            started_at: None,
            finished_at: None,
        }
    }

    #[test]
    fn test_valid_job_row_parses_successfully() {
        let row = valid_job_row();
        let job = row_to_job(row).expect("Failed to parse valid row");
        assert_eq!(job.revision(), 1);
        assert_eq!(job.id().to_string(), "00000000-0000-0000-0000-000000000001");
    }

    #[test]
    fn test_invalid_job_id() {
        let mut row = valid_job_row();
        row.id = "invalid-uuid".to_string();
        let result = row_to_job(row);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "id"
        ));
    }

    #[test]
    fn test_invalid_project_id() {
        let mut row = valid_job_row();
        row.project_id = "invalid-uuid".to_string();
        let result = row_to_job(row);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "project_id"
        ));
    }

    #[test]
    fn test_invalid_revision_out_of_bounds() {
        let mut row = valid_job_row();
        row.revision = 0;
        let result = row_to_job(row);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "revision"
        ));

        let mut row2 = valid_job_row();
        row2.revision = -1;
        let result = row_to_job(row2);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "revision"
        ));
    }

    #[test]
    fn test_invalid_json_fields() {
        let mut row = valid_job_row();
        row.kind = "UnknownKind".to_string();
        let result = row_to_job(row);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "kind"
        ));

        let mut row = valid_job_row();
        row.status = "UnknownStatus".to_string();
        let result = row_to_job(row);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "status"
        ));

        let mut row = valid_job_row();
        row.progress_json = "{invalid_json}".to_string();
        let result = row_to_job(row);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "progress_json"
        ));
    }

    #[test]
    fn test_invalid_datetime() {
        let mut row = valid_job_row();
        row.created_at = "not-a-date".to_string();
        let result = row_to_job(row);
        assert!(matches!(
            result,
            Err(PortError::InvalidStoredData { field, .. }) if field == "created_at"
        ));
    }

    #[test]
    fn test_job_to_row_values() {
        let row = valid_job_row();
        let job = row_to_job(valid_job_row()).unwrap();
        let row_values = job_to_row_values(&job).unwrap();
        assert_eq!(row_values.id, row.id);
        assert_eq!(row_values.project_id, row.project_id);
    }
}
