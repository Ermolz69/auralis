use domain::project::{Project, ProjectId, ProjectSnapshot};
use ports::error::PortError;
use std::str::FromStr;

use super::project_row::ProjectRow;

pub fn row_to_project(row: ProjectRow) -> Result<Project, PortError> {
    let id = parse_project_id(&row.id)?;

    let title = row.title;

    let status = parse_json_field(&format!("\"{}\"", row.status), &row.id, "status")?;

    let source = row
        .source_json
        .map(|s| parse_json_field(&s, &row.id, "source_json"))
        .transpose()?;
    let metadata = row
        .metadata_json
        .map(|s| parse_json_field(&s, &row.id, "metadata_json"))
        .transpose()?;
    let source_language = row
        .source_language
        .map(|s| parse_json_field(&s, &row.id, "source_language"))
        .transpose()?;
    let target_language = row
        .target_language
        .map(|s| parse_json_field(&s, &row.id, "target_language"))
        .transpose()?;
    let transcript = row
        .transcript_json
        .map(|s| parse_json_field(&s, &row.id, "transcript_json"))
        .transpose()?;

    let active_job_id = row
        .active_job_id
        .map(|id_str| parse_job_id(&id_str, &row.id, "active_job_id"))
        .transpose()?;

    let last_terminal_job_id = row
        .last_terminal_job_id
        .map(|id_str| parse_job_id(&id_str, &row.id, "last_terminal_job_id"))
        .transpose()?;

    let created_at = parse_datetime(&row.created_at, &row.id, "created_at")?;
    let updated_at = parse_datetime(&row.updated_at, &row.id, "updated_at")?;

    let snapshot = ProjectSnapshot {
        id,
        title,
        status,
        source,
        metadata,
        source_language,
        target_language,
        transcript,
        active_job_id,
        last_terminal_job_id,
        created_at,
        updated_at,
    };

    Project::from_snapshot(snapshot).map_err(|e| PortError::InvalidStoredData {
        entity_type: "project".to_string(),
        entity_id: row.id.clone(),
        field: "domain_validation".to_string(),
        message: format!("Failed to build project from snapshot: {:?}", e),
    })
}

pub fn project_to_row_values(project: &Project) -> Result<ProjectRow, PortError> {
    use super::helpers::serialize_json;

    let snapshot = project.to_snapshot();

    let status = serialize_json(&snapshot.status, "status")?
        .trim_matches('"')
        .to_string();

    let source_json = snapshot
        .source
        .map(|s| serialize_json(&s, "source"))
        .transpose()?;
    let metadata_json = snapshot
        .metadata
        .map(|s| serialize_json(&s, "metadata"))
        .transpose()?;
    let source_language = snapshot
        .source_language
        .map(|s| serialize_json(&s, "source_language"))
        .transpose()?;
    let target_language = snapshot
        .target_language
        .map(|s| serialize_json(&s, "target_language"))
        .transpose()?;
    let transcript_json = snapshot
        .transcript
        .map(|s| serialize_json(&s, "transcript"))
        .transpose()?;

    Ok(ProjectRow {
        id: snapshot.id.to_string(),
        title: snapshot.title,
        status,
        source_json,
        metadata_json,
        source_language,
        target_language,
        transcript_json,
        active_job_id: snapshot.active_job_id.map(|id| id.to_string()),
        last_terminal_job_id: snapshot.last_terminal_job_id.map(|id| id.to_string()),
        created_at: snapshot.created_at.to_rfc3339(),
        updated_at: snapshot.updated_at.to_rfc3339(),
    })
}

fn parse_project_id(value: &str) -> Result<ProjectId, PortError> {
    ProjectId::from_str(value).map_err(|e| PortError::InvalidStoredData {
        entity_type: "project".to_string(),
        entity_id: value.to_string(),
        field: "id".to_string(),
        message: format!("Failed to parse project id: {}", e),
    })
}

fn parse_job_id(
    value: &str,
    project_id: &str,
    field: &'static str,
) -> Result<domain::job::JobId, PortError> {
    domain::job::JobId::from_str(value).map_err(|e| PortError::InvalidStoredData {
        entity_type: "project".to_string(),
        entity_id: project_id.to_string(),
        field: field.to_string(),
        message: format!("Failed to parse job id `{}`: {}", value, e),
    })
}

fn parse_json_field<T: serde::de::DeserializeOwned>(
    value: &str,
    project_id: &str,
    field: &'static str,
) -> Result<T, PortError> {
    super::helpers::deserialize_json(value, field).map_err(|e| PortError::InvalidStoredData {
        entity_type: "project".to_string(),
        entity_id: project_id.to_string(),
        field: field.to_string(),
        message: format!("Failed to deserialize {}: {}", field, e),
    })
}

fn parse_datetime(
    value: &str,
    project_id: &str,
    field: &'static str,
) -> Result<chrono::DateTime<chrono::Utc>, PortError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| PortError::InvalidStoredData {
            entity_type: "project".to_string(),
            entity_id: project_id.to_string(),
            field: field.to_string(),
            message: format!("Failed to parse datetime: {}", e),
        })
}
