use ports::error::PortError;
use serde::{Deserialize, Serialize};

pub fn serialize_enum<T: Serialize>(value: &T, field: &str) -> Result<String, PortError> {
    serde_json::to_string(value)
        .map(|s| s.trim_matches('"').to_string())
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to serialize {}: {}", field, e),
        })
}

pub fn serialize_json<T: Serialize>(value: &T, field: &str) -> Result<String, PortError> {
    serde_json::to_string(value).map_err(|e| PortError::Unexpected {
        message: format!("Failed to serialize {}: {}", field, e),
    })
}

pub fn deserialize_json<'a, T: Deserialize<'a>>(
    json: &'a str,
    field: &str,
) -> Result<T, PortError> {
    serde_json::from_str(json).map_err(|e| PortError::Unexpected {
        message: format!("Failed to deserialize {}: {}", field, e),
    })
}
