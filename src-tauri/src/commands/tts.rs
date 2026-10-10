use application::usecases::speech_catalog::SpeechCatalogUseCase;
use ports::speech_catalog::SpeechCatalogSnapshot;
use std::sync::Arc;
use tauri::{State, command};

use crate::dto::error::{CommandError, parse_project_id};

#[command]
pub async fn get_speech_catalog_cmd(
    project_id: String,
    catalog: State<'_, Arc<SpeechCatalogUseCase>>,
) -> Result<SpeechCatalogSnapshot, CommandError> {
    catalog
        .execute(&parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}
