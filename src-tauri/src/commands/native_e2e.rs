use crate::dto::error::CommandError;
use tauri::command;

#[cfg(feature = "native-e2e")]
const ALLOWED_CHECKPOINTS: &[&str] = &[
    "react-started",
    "project-created",
    "listener-ready",
    "source-imported",
    "pipeline-paused",
    "cancelling-observed",
    "cancelled-returned",
    "project-cancelled",
    "ytdlp-import-returned",
    "ytdlp-project-ready",
];

#[command]
pub async fn native_e2e_checkpoint_cmd(checkpoint: String) -> Result<(), CommandError> {
    #[cfg(feature = "native-e2e")]
    {
        if !ALLOWED_CHECKPOINTS.contains(&checkpoint.as_str()) {
            return Err(CommandError::Validation(
                "Unknown native E2E checkpoint".to_string(),
            ));
        }
        crate::bootstrap::record_native_e2e_checkpoint(&checkpoint);
        Ok(())
    }

    #[cfg(not(feature = "native-e2e"))]
    {
        let _ = checkpoint;
        Err(CommandError::NotFound("Command unavailable".to_string()))
    }
}

#[command]
pub async fn native_e2e_pipeline_pause_reached_cmd() -> Result<bool, CommandError> {
    #[cfg(feature = "native-e2e")]
    {
        Ok(
            crate::bootstrap::native_e2e_subtitle_source::NativeE2ePausedSubtitleSource::pause_reached(),
        )
    }

    #[cfg(not(feature = "native-e2e"))]
    Err(CommandError::NotFound("Command unavailable".to_string()))
}
