use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::artifact::ArtifactDto;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationFormatDto;
use application::usecases::translation::ImportTranslationSourceRequest;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn import_translation_source_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    path: String,
    format: TranslationFormatDto,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<ArtifactDto, CommandError> {
    crate::observability::command::observe("import_translation_source_cmd", async {
        let artifact = usecases
            .import_source
            .execute(ImportTranslationSourceRequest {
                project_id: parse_project_id(&project_id)?,
                path: PathBuf::from(path),
                format: format.into(),
            })
            .await
            .map_err(CommandError::from)?;
        Ok(ArtifactDto::from(&artifact))
    })
    .await
}
