use crate::dto::{error::CommandError, translation::EditHistoricalTranslationResultDto};

#[cfg(feature = "native-e2e")]
const REQUEST_PATH_ENV: &str = "AURALIS_NATIVE_E2E_HISTORICAL_REQUEST_PATH";
#[cfg(feature = "native-e2e")]
const RELEASE_PATH_ENV: &str = "AURALIS_NATIVE_E2E_HISTORICAL_RETRY_RELEASE_PATH";

#[cfg(feature = "native-e2e")]
pub(super) async fn capture(
    input: &EditHistoricalTranslationResultDto,
) -> Result<(), CommandError> {
    if input.base_result_id == input.expected_head_result_id {
        return Ok(());
    }
    let Some(path) = std::env::var_os(REQUEST_PATH_ENV) else {
        return Ok(());
    };
    let bytes =
        serde_json::to_vec(input).map_err(|error| CommandError::Internal(error.to_string()))?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|error| CommandError::Io(error.to_string()))
}

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn native_e2e_historical_retry_input_cmd(
    request: tauri::ipc::Request<'_>,
) -> Result<Option<EditHistoricalTranslationResultDto>, CommandError> {
    crate::observability::command::observe("native_e2e_historical_retry_input_cmd", async {
        #[cfg(feature = "native-e2e")]
        {
            let Some(release) = std::env::var_os(RELEASE_PATH_ENV) else {
                return Ok(None);
            };
            if !tokio::fs::try_exists(release)
                .await
                .map_err(|error| CommandError::Io(error.to_string()))?
            {
                return Ok(None);
            }
            let Some(path) = std::env::var_os(REQUEST_PATH_ENV) else {
                return Ok(None);
            };
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|error| CommandError::Io(error.to_string()))?;
            serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| CommandError::Validation(error.to_string()))
        }
        #[cfg(not(feature = "native-e2e"))]
        Err(CommandError::NotFound("Command unavailable".into()))
    })
    .await
}
