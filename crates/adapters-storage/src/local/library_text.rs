use async_trait::async_trait;
use ports::{error::PortError, library::LibraryTextReader};
use std::{io::Read, path::Path};

pub struct LocalLibraryTextReader;
#[async_trait]
impl LibraryTextReader for LocalLibraryTextReader {
    async fn read_text(&self, path: &str) -> Result<String, PortError> {
        let path = Path::new(path);
        if !path.is_absolute()
            || path
                .extension()
                .is_none_or(|e| !e.eq_ignore_ascii_case("txt"))
        {
            return Err(PortError::InvalidSource {
                message: "Choose an absolute UTF-8 TXT file".into(),
            });
        }
        let file = std::fs::File::open(path).map_err(|e| PortError::Io {
            message: e.to_string(),
        })?;
        let mut bytes = Vec::new();
        file.take(1_000_001)
            .read_to_end(&mut bytes)
            .map_err(|e| PortError::Io {
                message: e.to_string(),
            })?;
        if bytes.len() > 1_000_000 {
            return Err(PortError::InvalidSource {
                message: "TXT exceeds 1 MB".into(),
            });
        }
        let text = String::from_utf8(bytes).map_err(|_| PortError::InvalidSource {
            message: "TXT must use UTF-8 encoding".into(),
        })?;
        if text.contains('\0') {
            return Err(PortError::InvalidSource {
                message: "TXT contains a binary NUL character".into(),
            });
        }
        Ok(text.trim_start_matches('\u{feff}').to_owned())
    }
}
