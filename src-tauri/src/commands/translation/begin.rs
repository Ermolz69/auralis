use super::parse_translation_id;
use crate::bootstrap::translation::{EXPERIMENTAL_CHECKED_CHINESE_PROFILE, TranslationUseCases};
use crate::dto::error::{CommandError, parse_artifact_id, parse_project_id};
use crate::dto::translation::{BeginTranslationRunDto, TranslationLinkDto};
use application::usecases::translation::BeginTranslationRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn begin_translation_run_cmd(
    request: tauri::ipc::Request<'_>,
    input: BeginTranslationRunDto,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationLinkDto, CommandError> {
    crate::observability::command::observe("begin_translation_run_cmd", async {
        let link = usecases
            .begin
            .execute(BeginTranslationRequest {
                project_id: parse_project_id(&input.project_id)?,
                source_artifact_id: parse_artifact_id(&input.source_artifact_id)?,
                format: input.format.into(),
                source_language: input.source_language.as_str().into(),
                translation_id: parse_translation_id(&input.translation_id)?,
                run_id: parse_translation_id(&input.run_id)?,
                profile_manifest_json: EXPERIMENTAL_CHECKED_CHINESE_PROFILE.into(),
                created_at: domain::chrono::Utc::now(),
            })
            .await
            .map_err(CommandError::from)?;
        Ok(link.into())
    })
    .await
}
