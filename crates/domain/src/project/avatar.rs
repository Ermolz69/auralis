use crate::error::DomainError;
use base64::{Engine, engine::general_purpose::STANDARD};

pub const MAX_AVATAR_BYTES: usize = 1024 * 1024;
pub const MAX_AVATAR_DATA_URL_LENGTH: usize = 1_398_200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAvatar(String);

impl ProjectAvatar {
    pub fn new(data_url: String) -> Result<Self, DomainError> {
        let invalid = || {
            DomainError::ValidationError(
                "Avatar must be a PNG, JPEG, WebP or GIF image up to 1 MiB".into(),
            )
        };
        if data_url.len() > MAX_AVATAR_DATA_URL_LENGTH {
            return Err(invalid());
        }
        let (header, encoded) = data_url.split_once(',').ok_or_else(invalid)?;
        let bytes = STANDARD.decode(encoded).map_err(|_| invalid())?;
        if bytes.len() > MAX_AVATAR_BYTES {
            return Err(invalid());
        }
        let valid = match header {
            "data:image/png;base64" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "data:image/jpeg;base64" => bytes.starts_with(b"\xff\xd8\xff"),
            "data:image/gif;base64" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
            "data:image/webp;base64" => {
                bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
            }
            _ => false,
        };
        if !valid {
            return Err(invalid());
        }
        Ok(Self(data_url))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
