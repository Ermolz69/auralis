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
    let id = ArtifactId::from_str(&row.id).map_err(|e| PortError::Unexpected {
        message: format!("Invalid artifact ID {}: {}", row.id, e),
    })?;

    let kind_json = serde_json::Value::String(row.kind);
    let kind: ArtifactKind =
        serde_json::from_value(kind_json).map_err(|e| PortError::Unexpected {
            message: format!("Invalid artifact kind: {}", e),
        })?;

    let location = match row.location_kind.as_str() {
        "LocalPath" => ArtifactLocation::LocalPath(row.location_value),
        "StorageKey" => ArtifactLocation::StorageKey(row.location_value),
        other => {
            return Err(PortError::Unexpected {
                message: format!("Invalid location kind: {}", other),
            });
        }
    };

    let state_json = serde_json::Value::String(row.state);
    let state: domain::media::ArtifactState =
        serde_json::from_value(state_json).map_err(|e| PortError::Unexpected {
            message: format!("Invalid artifact state: {}", e),
        })?;

    let created_at = chrono::DateTime::parse_from_rfc3339(&row.created_at)
        .map_err(|e| PortError::Unexpected {
            message: format!("Invalid created_at date {}: {}", row.created_at, e),
        })?
        .with_timezone(&chrono::Utc);

    let updated_at = chrono::DateTime::parse_from_rfc3339(&row.updated_at)
        .map_err(|e| PortError::Unexpected {
            message: format!("Invalid updated_at date {}: {}", row.updated_at, e),
        })?
        .with_timezone(&chrono::Utc);

    let ready_at = row
        .ready_at
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map_err(|e| PortError::Unexpected {
                    message: format!("Invalid ready_at date {}: {}", s, e),
                })
                .map(|dt| dt.with_timezone(&chrono::Utc))
        })
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
