use super::{
    ExportTranslationResultReceipt, ExportTranslationResultRequest, VerifyTranslationHistoryUseCase,
};
use crate::error::ApplicationError;
use ports::error::PortError;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ExportTranslationResultUseCase {
    verifier: Arc<VerifyTranslationHistoryUseCase>,
    owned_root: PathBuf,
}

impl ExportTranslationResultUseCase {
    pub fn new(verifier: Arc<VerifyTranslationHistoryUseCase>, owned_root: PathBuf) -> Self {
        Self {
            verifier,
            owned_root,
        }
    }

    pub async fn execute(
        &self,
        request: ExportTranslationResultRequest,
    ) -> Result<ExportTranslationResultReceipt, ApplicationError> {
        let verified = self
            .verifier
            .execute(
                request.project_id,
                request.translation_id,
                request.result_id,
            )
            .await?;
        if verified.result.output_sha256 != request.expected_result_sha256 {
            return Err(conflict("result digest changed before export").into());
        }
        let format = match verified.link.source_format.as_str() {
            "srt" => "srt",
            "vtt" => "vtt",
            _ => return Err(invalid("unsupported export format").into()),
        };
        let destination = request.destination;
        let size_bytes = u64::try_from(verified.result.output_bytes.len())
            .map_err(|_| invalid("translation output is too large"))?;
        let receipt = ExportTranslationResultReceipt {
            result_id: verified.result.result_id,
            result_sha256: verified.result.output_sha256,
            size_bytes,
            destination: destination.clone(),
        };
        let bytes = verified.result.output_bytes;
        let owned_root = self.owned_root.clone();
        tokio::task::spawn_blocking(move || export_new(&owned_root, &destination, format, &bytes))
            .await
            .map_err(|error| ApplicationError::Unexpected(error.to_string()))??;
        Ok(receipt)
    }
}

fn export_new(
    owned_root: &Path,
    destination: &Path,
    format: &str,
    bytes: &[u8],
) -> Result<(), PortError> {
    if !destination.is_absolute()
        || !destination
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(format))
    {
        return Err(invalid(
            "export destination must be an absolute path with the source format",
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("export destination has no parent directory"))?;
    let parent =
        std::fs::canonicalize(parent).map_err(|error| io("resolve export directory", error))?;
    let filename = destination
        .file_name()
        .ok_or_else(|| invalid("export destination has no filename"))?;
    let owned_root = std::fs::canonicalize(owned_root)
        .map_err(|error| io("resolve application data directory", error))?;
    if parent.starts_with(owned_root) {
        return Err(invalid("export destination is inside application data"));
    }
    let published = parent.join(filename);
    let mut staged = tempfile::NamedTempFile::new_in(&parent)
        .map_err(|error| io("stage translation export", error))?;
    staged
        .write_all(bytes)
        .map_err(|error| io("write translation export", error))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|error| io("persist translation export", error))?;
    staged.persist_noclobber(&published).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            conflict("export destination already exists")
        } else {
            io("publish translation export", error.error)
        }
    })?;
    Ok(())
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation export".into(),
        message: message.into(),
    }
}

fn io(operation: &str, error: std::io::Error) -> PortError {
    PortError::Io {
        message: format!("{operation}: {error}"),
    }
}
