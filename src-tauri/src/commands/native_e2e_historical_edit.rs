use crate::dto::error::CommandError;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn native_e2e_historical_edit_held_cmd(
    request: tauri::ipc::Request<'_>,
) -> Result<bool, CommandError> {
    crate::observability::command::observe("native_e2e_historical_edit_held_cmd", async {
        #[cfg(feature = "native-e2e")]
        {
            let Some(path) = std::env::var_os("AURALIS_NATIVE_E2E_HISTORICAL_COMMITTED_PATH")
            else {
                return Ok(false);
            };
            match tokio::fs::read_to_string(path).await {
                Ok(value) => Ok(value.split(':').count() == 2
                    && value
                        .split(':')
                        .all(|part| uuid::Uuid::parse_str(part).is_ok())),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(CommandError::Internal(error.to_string())),
            }
        }
        #[cfg(not(feature = "native-e2e"))]
        Err(CommandError::NotFound("Command unavailable".into()))
    })
    .await
}
