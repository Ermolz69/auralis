use crate::error::{ApplicationError, PortErrorKind};
use domain::outbox::{OutboxMessage, OutboxPayload};

pub(super) fn error_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::Port(error) => PortErrorKind::from_error(error).to_string(),
        _ => "Application".into(),
    }
}

pub(super) fn completed(
    message: &OutboxMessage,
    outcome: &'static str,
    error_code: &str,
    started: std::time::Instant,
) {
    let operation = match &message.payload {
        OutboxPayload::FinalizeStagedArtifact { .. } => "finalize_artifact",
        OutboxPayload::DeleteStorageKey { .. } => "delete_storage_key",
        OutboxPayload::DeleteProjectArtifactDir { .. } => "delete_project_directory",
        OutboxPayload::DeleteWorkspaceAllocation { .. } => "delete_workspace",
        OutboxPayload::HandleTerminalJobState { .. } => "terminalize_job",
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    if outcome == "completed" {
        tracing::info!(event_name = "outbox_attempt_completed", operation, operation_id = %message.id,
            attempt = message.attempts + 1, outcome, error_code, duration_ms);
    } else {
        tracing::warn!(event_name = "outbox_attempt_completed", operation, operation_id = %message.id,
            attempt = message.attempts + 1, outcome, error_code, duration_ms);
    }
}
