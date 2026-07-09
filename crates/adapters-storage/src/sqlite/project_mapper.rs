use std::str::FromStr;
use domain::project::{Project, ProjectId, ProjectSnapshot};
use ports::error::PortError;

use super::project_row::ProjectRow;

pub fn row_to_project(row: ProjectRow) -> Result<Project, PortError> {
    let id = ProjectId::from_str(&row.id).map_err(|e| PortError::Unexpected {
        message: format!("Failed to parse project id `{}`: {}", row.id, e),
    })?;

    let title = row.title;
    
    let status = parse_json(&format!("\"{}\"", row.status), "status")?;

    let source = row.source_json.map(|s| parse_json(&s, "source_json")).transpose()?;
    let metadata = row.metadata_json.map(|s| parse_json(&s, "metadata_json")).transpose()?;
    let source_language = row.source_language.map(|s| parse_json(&s, "source_language")).transpose()?;
    let target_language = row.target_language.map(|s| parse_json(&s, "target_language")).transpose()?;
    let transcript = row.transcript_json.map(|s| parse_json(&s, "transcript_json")).transpose()?;
    
    let artifacts = parse_json(&row.artifacts_json, "artifacts_json")?;

    let created_at = parse_datetime(&row.created_at, "created_at")?;
    let updated_at = parse_datetime(&row.updated_at, "updated_at")?;

    let snapshot = ProjectSnapshot {
        id,
        title,
        status,
        source,
        metadata,
        source_language,
        target_language,
        transcript,
        artifacts,
        created_at,
        updated_at,
    };

    Project::from_snapshot(snapshot).map_err(|e| PortError::Unexpected {
        message: format!("Failed to build project from snapshot: {:?}", e),
    })
}

pub fn project_to_row_values(project: &Project) -> Result<ProjectRow, PortError> {
    let snapshot = project.to_snapshot();

    let status = serde_json::to_string(&snapshot.status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    let source_json = snapshot.source.map(|s| serde_json::to_string(&s).unwrap());
    let metadata_json = snapshot.metadata.map(|s| serde_json::to_string(&s).unwrap());
    let source_language = snapshot.source_language.map(|s| serde_json::to_string(&s).unwrap());
    let target_language = snapshot.target_language.map(|s| serde_json::to_string(&s).unwrap());
    let transcript_json = snapshot.transcript.map(|s| serde_json::to_string(&s).unwrap());

    let artifacts_json = serde_json::to_string(&snapshot.artifacts).map_err(|e| PortError::Unexpected {
        message: format!("Failed to serialize artifacts_json: {}", e),
    })?;

    Ok(ProjectRow {
        id: snapshot.id.to_string(),
        title: snapshot.title,
        status,
        source_json,
        metadata_json,
        source_language,
        target_language,
        transcript_json,
        artifacts_json,
        created_at: snapshot.created_at.to_rfc3339(),
        updated_at: snapshot.updated_at.to_rfc3339(),
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(value: &str, field: &str) -> Result<T, PortError> {
    serde_json::from_str(value).map_err(|e| PortError::Unexpected {
        message: format!("Failed to deserialize field `{}`: {}", field, e),
    })
}

fn parse_datetime(value: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, PortError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to parse field `{}` as datetime: {}", field, e),
        })
}
