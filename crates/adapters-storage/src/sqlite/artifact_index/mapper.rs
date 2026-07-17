use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation};
use domain::project::ProjectId;
use ports::error::PortError;
use std::str::FromStr;

use super::row::ArtifactRow;
use super::serialization::{artifact_kind_to_db, artifact_state_to_db};

pub fn artifact_to_row_values(
    project_id: &ProjectId,
    artifact: &Artifact,
) -> Result<ArtifactRow, PortError> {
    let (location_kind, location_value) = match &artifact.location {
        ArtifactLocation::LocalPath(path) => ("LocalPath".to_string(), path.clone()),
        ArtifactLocation::StorageKey(key) => ("StorageKey".to_string(), key.clone()),
    };

    let kind = artifact_kind_to_db(&artifact.kind)?;
    let state = artifact_state_to_db(&artifact.state)?;

    Ok(ArtifactRow {
        id: artifact.id.to_string(),
        project_id: project_id.to_string(),
        kind,
        location_kind,
        location_value,
        size_bytes: artifact.size_bytes.map(|s| s as i64),
        state,
        created_at: artifact.created_at.to_rfc3339(),
        updated_at: artifact.updated_at.to_rfc3339(),
        ready_at: artifact.ready_at.map(|dt| dt.to_rfc3339()),
    })
}

pub fn row_to_artifact(row: ArtifactRow) -> Result<Artifact, PortError> {
    let id = parse_artifact_id(&row.id)?;

    let kind_json = serde_json::Value::String(row.kind);
    let kind: ArtifactKind =
        serde_json::from_value(kind_json).map_err(|e| PortError::InvalidStoredData {
            entity_type: "artifact".to_string(),
            entity_id: row.id.clone(),
            field: "kind".to_string(),
            message: format!("Invalid artifact kind: {}", e),
        })?;

    let location = match row.location_kind.as_str() {
        "LocalPath" => ArtifactLocation::LocalPath(row.location_value),
        "StorageKey" => ArtifactLocation::StorageKey(row.location_value),
        other => {
            return Err(PortError::InvalidStoredData {
                entity_type: "artifact".to_string(),
                entity_id: row.id.clone(),
                field: "location_kind".to_string(),
                message: format!("Invalid location kind: {}", other),
            });
        }
    };

    let state_json = serde_json::Value::String(row.state);
    let state: domain::media::ArtifactState =
        serde_json::from_value(state_json).map_err(|e| PortError::InvalidStoredData {
            entity_type: "artifact".to_string(),
            entity_id: row.id.clone(),
            field: "state".to_string(),
            message: format!("Invalid artifact state: {}", e),
        })?;

    let created_at = parse_datetime(&row.created_at, &row.id, "created_at")?;
    let updated_at = parse_datetime(&row.updated_at, &row.id, "updated_at")?;

    let ready_at = row
        .ready_at
        .map(|s| parse_datetime(&s, &row.id, "ready_at"))
        .transpose()?;

    Ok(Artifact {
        id,
        kind,
        location,
        size_bytes: row.size_bytes.map(|s| s as u64),
        state,
        created_at,
        updated_at,
        ready_at,
    })
}

fn parse_artifact_id(value: &str) -> Result<ArtifactId, PortError> {
    ArtifactId::from_str(value).map_err(|e| PortError::InvalidStoredData {
        entity_type: "artifact".to_string(),
        entity_id: value.to_string(),
        field: "id".to_string(),
        message: format!("Invalid artifact ID: {}", e),
    })
}

fn parse_datetime(
    value: &str,
    artifact_id: &str,
    field: &'static str,
) -> Result<chrono::DateTime<chrono::Utc>, PortError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|e| PortError::InvalidStoredData {
            entity_type: "artifact".to_string(),
            entity_id: artifact_id.to_string(),
            field: field.to_string(),
            message: format!("Invalid datetime {}: {}", value, e),
        })
        .map(|dt| dt.with_timezone(&chrono::Utc))
}
