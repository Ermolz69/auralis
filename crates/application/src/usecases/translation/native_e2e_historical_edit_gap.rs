use crate::error::ApplicationError;
use uuid::Uuid;

const MARKER_PATH_ENV: &str = "AURALIS_NATIVE_E2E_HISTORICAL_COMMITTED_PATH";

pub(super) async fn hold_if_requested(
    run_id: Uuid,
    result_id: Uuid,
) -> Result<(), ApplicationError> {
    let Some(path) = std::env::var_os(MARKER_PATH_ENV) else {
        return Ok(());
    };
    tokio::fs::write(path, format!("{run_id}:{result_id}"))
        .await
        .map_err(|error| ApplicationError::Unexpected(error.to_string()))?;
    std::future::pending::<()>().await;
    Ok(())
}
