use super::parse_translation_id;
use crate::bootstrap::selected_translation_runtime::SelectedTranslationRuntime;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationStartDto;
use crate::state::ManagedJobRuntime;
use application::usecases::translation::{
    ScheduleManagedTranslationUseCase, TranslationRunRequest,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn start_translation_run_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
    media: State<'_, Arc<dyn ports::media_project::MediaProjectStore>>,
    selected_runtime: State<'_, SelectedTranslationRuntime>,
    runtime: State<'_, ManagedJobRuntime>,
) -> Result<TranslationStartDto, CommandError> {
    crate::observability::command::observe("start_translation_run_cmd", async {
        media
            .ensure(&project_id)
            .await
            .map_err(application::error::ApplicationError::from)?;
        let selected = selected_runtime.read().await;
        let runtime_port = selected.as_ref().ok_or_else(|| {
            CommandError::NotFound("Translation model runtime is not installed".into())
        })?;
        let scheduler = ScheduleManagedTranslationUseCase::new(
            usecases.managed_for_runtime(runtime_port.clone()),
            runtime.0.clone(),
            usecases.jobs.clone(),
            usecases.run.clone(),
        );
        let (job, run_id) = scheduler
            .execute(TranslationRunRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
            })
            .await
            .map_err(CommandError::from)?;
        Ok(TranslationStartDto::accepted(job, run_id))
    })
    .await
}
