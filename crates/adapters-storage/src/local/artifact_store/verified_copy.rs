use super::{path_guard::ensure_safe_parent, resolver::resolve_storage_key};
use crate::local::storage_key::make_storage_key;
use domain::media::{Artifact, ArtifactId, ArtifactLocation, ArtifactState};
use ports::{
    PortError,
    cancellation::{CancelOnDrop, CancellationToken},
    storage::{StagedArtifact, VerifiedArtifactCopy},
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(test)]
mod tests;

pub(super) async fn stage(
    base: PathBuf,
    request: VerifiedArtifactCopy,
    cancel: CancellationToken,
) -> Result<StagedArtifact, PortError> {
    let cancel = cancel.child_token();
    let guard = CancelOnDrop(cancel.clone());
    let result = tokio::spawn(async move { stage_owned(base, request, cancel).await })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?;
    drop(guard);
    result
}

async fn stage_owned(
    base: PathBuf,
    request: VerifiedArtifactCopy,
    cancel: CancellationToken,
) -> Result<StagedArtifact, PortError> {
    let deadline = Instant::now() + Duration::from_secs(120);
    check(&cancel, deadline)?;
    let metadata = tokio::fs::symlink_metadata(&request.source_path)
        .await
        .map_err(io_error)?;
    if !base.is_absolute()
        || !request.source_path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || request.max_bytes == 0
        || request.max_bytes > 1_u64 << 32
        || metadata.len() == 0
        || metadata.len() > request.max_bytes
    {
        return Err(invalid(
            "Verified staging requires a bounded regular source and an absolute managed root",
        ));
    }
    let extension = request
        .filename_hint
        .as_deref()
        .and_then(|hint| Path::new(hint).extension())
        .or_else(|| request.source_path.extension())
        .and_then(|extension| extension.to_str())
        .unwrap_or("bin");
    if extension.is_empty()
        || extension.len() > 16
        || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(invalid("Verified staging filename extension is invalid"));
    }
    let artifact_id = ArtifactId::new();
    let final_key = make_storage_key(&request.project_id, &artifact_id, &request.kind, extension);
    let staging_key = format!(
        ".staging/{}/{}.{}",
        uuid::Uuid::new_v4(),
        artifact_id,
        extension
    );
    let destination = resolve_storage_key(&base, &staging_key)?;
    ensure_safe_parent(&base, &destination).await?;
    let mut output = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .await
        .map_err(io_error)?;
    let copied = copy(&request, &mut output, metadata.len(), &cancel, deadline).await;
    drop(output);
    let size = match copied {
        Ok(size) => size,
        Err(error) => {
            cleanup(&destination).await?;
            return Err(error);
        }
    };
    if let Err(error) = check(&cancel, deadline) {
        cleanup(&destination).await?;
        return Err(error);
    }
    let now = domain::chrono::Utc::now();
    Ok(StagedArtifact {
        artifact: Artifact {
            id: artifact_id,
            kind: request.kind,
            location: ArtifactLocation::StorageKey(final_key.clone()),
            size_bytes: Some(size),
            state: ArtifactState::PendingFinalize,
            created_at: now,
            updated_at: now,
            ready_at: None,
        },
        staging_key,
        final_key,
        size_bytes: size,
    })
}

async fn copy(
    request: &VerifiedArtifactCopy,
    output: &mut tokio::fs::File,
    expected_size: u64,
    cancel: &CancellationToken,
    deadline: Instant,
) -> Result<u64, PortError> {
    let mut input = tokio::fs::File::open(&request.source_path)
        .await
        .map_err(io_error)?;
    let transferred = transfer(
        &mut input,
        output,
        expected_size,
        &request.expected_sha256,
        cancel,
        deadline,
    )
    .await;
    let flushed = output.flush().await.map_err(io_error);
    let synced = if transferred.is_ok() && flushed.is_ok() {
        output.sync_all().await.map_err(io_error)
    } else {
        Ok(())
    };
    drop(input);
    transferred.and_then(|size| {
        flushed?;
        synced?;
        Ok(size)
    })
}

async fn transfer(
    input: &mut (impl AsyncRead + Unpin),
    output: &mut (impl AsyncWrite + Unpin),
    expected_size: u64,
    expected_sha256: &domain::speech::Sha256Digest,
    cancel: &CancellationToken,
    deadline: Instant,
) -> Result<u64, PortError> {
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 65_536];
    loop {
        check(cancel, deadline)?;
        let read = input.read(&mut buffer).await.map_err(io_error)?;
        if read == 0 {
            break;
        }
        check(cancel, deadline)?;
        bytes = bytes
            .checked_add(read as u64)
            .filter(|size| *size <= expected_size)
            .ok_or_else(|| invalid("Verified source grew beyond its immutable size"))?;
        output.write_all(&buffer[..read]).await.map_err(io_error)?;
        digest.update(&buffer[..read]);
    }
    if bytes != expected_size || format!("{:x}", digest.finalize()) != expected_sha256.as_str() {
        return Err(invalid(
            "Verified staged copy differs from its source digest or size",
        ));
    }
    Ok(bytes)
}

async fn cleanup(path: &Path) -> Result<(), PortError> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(PortError::CleanupFailed {
                resource: "verified_artifact_staging".into(),
                message: error.to_string(),
            });
        }
    }
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::remove_dir(parent).await;
    }
    Ok(())
}

fn check(cancel: &CancellationToken, deadline: Instant) -> Result<(), PortError> {
    if cancel.is_cancelled() {
        Err(PortError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(invalid(
            "Verified artifact staging exceeded its 120-second deadline",
        ))
    } else {
        Ok(())
    }
}
fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}
