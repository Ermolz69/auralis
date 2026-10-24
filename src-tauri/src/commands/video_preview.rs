use crate::{
    bootstrap::{paths::AppPaths, usecases::AppUseCases},
    dto::error::{CommandError, parse_artifact_id},
};
use application::usecases::artifact::resolve_path::ResolveArtifactPathRequest;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::io::AsyncReadExt;

static PREVIEW_CAPACITY: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(32);
static PREVIEW_QUEUE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tauri::command]
pub async fn get_video_preview_cmd(
    app: AppHandle,
    artifact_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
    paths: State<'_, AppPaths>,
) -> Result<String, CommandError> {
    crate::observability::command::observe("get_video_preview_cmd", async {
        let _slot = PREVIEW_CAPACITY.try_acquire().map_err(|_| {
            CommandError::Io(
                "Очередь превью заполнена. Повторите после завершения текущих превью.".into(),
            )
        })?;
        let _guard = PREVIEW_QUEUE.lock().await;
        let source = usecases
            .resolve_artifact_path
            .execute(ResolveArtifactPathRequest {
                artifact_id: parse_artifact_id(&artifact_id)?,
            })
            .await?;
        if !source.managed_ready {
            return Err(CommandError::Io(
                "Превью доступно только для готового файла".into(),
            ));
        }
        let mut file = tokio::fs::File::open(&source.absolute_path)
            .await
            .map_err(io)?;
        let mut hash = Sha256::new();
        hash.update(b"auralis-video-preview-v1:320:cpu");
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            let count = file.read(&mut buffer).await.map_err(io)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        let cache = paths.root().join("cache/video-previews");
        tokio::fs::create_dir_all(&cache).await.map_err(io)?;
        let output = cache.join(format!("{:x}.png", hash.finalize()));
        if !verified_preview(&output).await {
            let temporary = cache.join(format!("{}.png", uuid::Uuid::new_v4()));
            let mut last = "FFmpeg не найден".to_string();
            for executable in crate::bootstrap::media_tools::resolve_ffmpeg_candidates(&app) {
                let mut command = tokio::process::Command::new(executable);
                command
                    .args([
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-nostdin",
                        "-hwaccel",
                        "none",
                        "-threads",
                        "1",
                        "-i",
                    ])
                    .arg(&source.absolute_path)
                    .args([
                        "-frames:v",
                        "1",
                        "-vf",
                        "scale=320:-2",
                        "-threads",
                        "1",
                        "-y",
                    ])
                    .arg(&temporary)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .kill_on_drop(true);
                #[cfg(windows)]
                command.creation_flags(0x08000000);
                let mut child = match command.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        last = e.to_string();
                        continue;
                    }
                };
                match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait()).await {
                    Ok(Ok(status)) if status.success() && valid_png(&temporary).await => {
                        let bytes = tokio::fs::read(&temporary).await.map_err(io)?;
                        tokio::fs::rename(&temporary, &output).await.map_err(io)?;
                        tokio::fs::write(
                            output.with_extension("sha256"),
                            format!("{:x}", Sha256::digest(bytes)),
                        )
                        .await
                        .map_err(io)?;
                        break;
                    }
                    Ok(_) => {
                        last = "Не удалось прочитать кадр видео".into();
                        break;
                    }
                    Err(_) => {
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                        last = "Истекло время создания превью".into();
                        break;
                    }
                }
            }
            if !verified_preview(&output).await {
                let _ = tokio::fs::remove_file(temporary).await;
                return Err(CommandError::Io(last));
            }
        }
        app.asset_protocol_scope()
            .allow_file(&output)
            .map_err(|_| CommandError::Io("Не удалось открыть превью".into()))?;
        Ok(output.to_string_lossy().into_owned())
    })
    .await
}
async fn valid_png(path: &std::path::Path) -> bool {
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            bytes.len() > 45
                && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                && bytes.ends_with(b"IEND\xaeB`\x82")
                && bytes.len() < 2_000_000
        }
        Err(_) => false,
    }
}
fn io(error: std::io::Error) -> CommandError {
    CommandError::Io(error.to_string())
}

async fn verified_preview(path: &std::path::Path) -> bool {
    if !valid_png(path).await {
        return false;
    }
    match (
        tokio::fs::read(path).await,
        tokio::fs::read_to_string(path.with_extension("sha256")).await,
    ) {
        (Ok(bytes), Ok(expected)) => format!("{:x}", Sha256::digest(bytes)) == expected,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cache_rejects_partial_and_modified_images() -> Result<(), Box<dyn std::error::Error>> {
        let directory =
            std::env::temp_dir().join(format!("auralis-preview-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&directory).await?;
        let path = directory.join("frame.png");
        assert!(!verified_preview(&path).await);
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend([0; 50]);
        bytes.extend(b"IEND\xaeB`\x82");
        tokio::fs::write(&path, &bytes).await?;
        assert!(!verified_preview(&path).await);
        tokio::fs::write(
            path.with_extension("sha256"),
            format!("{:x}", Sha256::digest(&bytes)),
        )
        .await?;
        assert!(verified_preview(&path).await);
        bytes[20] = 1;
        tokio::fs::write(&path, &bytes).await?;
        assert!(!verified_preview(&path).await);
        tokio::fs::write(&path, &bytes[..30]).await?;
        assert!(!verified_preview(&path).await);
        tokio::fs::remove_dir_all(directory).await?;
        Ok(())
    }
}
