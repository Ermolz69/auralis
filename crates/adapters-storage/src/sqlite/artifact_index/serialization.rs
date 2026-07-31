use domain::media::{ArtifactKind, ArtifactState};
use ports::error::PortError;

pub fn artifact_kind_to_db(kind: &ArtifactKind) -> Result<String, PortError> {
    let kind_val = serde_json::to_value(kind).map_err(|e| PortError::Storage {
        operation: "artifact_kind_to_db",
        message: format!("Failed to serialize artifact kind: {}", e),
    })?;

    kind_val
        .as_str()
        .ok_or_else(|| PortError::Storage {
            operation: "artifact_kind_to_db",
            message: "Artifact kind is not a string".to_string(),
        })
        .map(|s| s.to_string())
}

pub fn artifact_state_to_db(state: &ArtifactState) -> Result<String, PortError> {
    let state_val = serde_json::to_value(state).map_err(|e| PortError::Storage {
        operation: "artifact_state_to_db",
        message: format!("Failed to serialize artifact state: {}", e),
    })?;

    state_val
        .as_str()
        .ok_or_else(|| PortError::Storage {
            operation: "artifact_state_to_db",
            message: "Artifact state is not a string".to_string(),
        })
        .map(|s| s.to_string())
}
