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

pub fn map_sqlite_error(context: &str, error: sqlx::Error) -> PortError {
    match error {
        sqlx::Error::Database(ref db_err) => {
            if let Some(code) = db_err.code() {
                // SQLite extended codes have the primary error code in the lowest 8 bits.
                if let Ok(extended_code) = code.parse::<i32>() {
                    let primary_code = extended_code & 0xFF;
                    if primary_code == 5 || primary_code == 6 {
                        return PortError::Busy {
                            message: format!("{}: Database is busy or locked", context),
                        };
                    }
                }
            }
            PortError::Unexpected {
                message: format!("{}: {}", context, error),
            }
        }
        _ => PortError::Unexpected {
            message: format!("{}: {}", context, error),
        },
    }
}
