use crate::dto::error::CommandError;

pub(crate) async fn observe<T>(
    operation: &'static str,
    action: impl std::future::Future<Output = Result<T, CommandError>>,
) -> Result<T, CommandError> {
    let started = std::time::Instant::now();
    let result = action.await;
    let duration_ms = started.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => tracing::info!(
            event_name = "command_completed",
            operation,
            outcome = "succeeded",
            duration_ms
        ),
        Err(error) => {
            let (outcome, error_code) = match error {
                CommandError::Conflict(_) => ("conflict", "CONFLICT"),
                CommandError::Cancelled(_) => ("cancelled", "CANCELLED"),
                CommandError::NotFound(_) => ("failed", "NOT_FOUND"),
                CommandError::Validation(_) => ("failed", "VALIDATION"),
                CommandError::Busy(_) => ("failed", "BUSY"),
                CommandError::Io(_) => ("failed", "IO"),
                CommandError::Repository(_) => ("failed", "REPOSITORY"),
                CommandError::RecoveryRequired(_) => ("failed", "RECOVERY_REQUIRED"),
                CommandError::Internal(_) => ("failed", "INTERNAL"),
            };
            tracing::warn!(
                event_name = "command_completed",
                operation,
                outcome,
                error_code,
                duration_ms
            );
        }
    }
    result
}
