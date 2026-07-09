use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation};
use domain::project::ProjectId;
use ports::error::PortError;
use std::str::FromStr;

use super::artifact_row::ArtifactRow;

pub fn artifact_to_row_values(
    project_id: &ProjectId,
    artifact: &Artifact,
) -> Result<ArtifactRow, PortError> {
    let (location_kind, location_value) = match &artifact.location {
        ArtifactLocation::LocalPath(path) => ("LocalPath".to_string(), path.clone()),
        ArtifactLocation::StorageKey(key) => ("StorageKey".to_string(), key.clone()),
    };

    let kind_val = serde_json::to_value(&artifact.kind).map_err(|e| PortError::Unexpected {
        message: format!("Failed to serialize artifact kind: {}", e),
    })?;

    let kind = kind_val
        .as_str()
        .ok_or_else(|| PortError::Unexpected {
            message: "Artifact kind is not a string".to_string(),
        })?
        .to_string();

    Ok(ArtifactRow {
        id: artifact.id.to_string(),
        project_id: project_id.to_string(),
        kind,
        location_kind,
        location_value,
        size_bytes: artifact.size_bytes.map(|s| s as i64),
        created_at: chrono::Utc::now().to_rfc3339(),
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

    Ok(Artifact {
        id,
        kind,
        location,
        size_bytes: row.size_bytes.map(|s| s as u64),
    })
}
