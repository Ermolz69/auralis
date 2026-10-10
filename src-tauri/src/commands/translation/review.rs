use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::{TranslationReviewDecisionDto, TranslationReviewViewDto};
use application::usecases::translation::{
    GetTranslationReviewRequest, RecordTranslationReviewRequest,
};
use ports::events::AppEventPublisher;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_translation_review_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    result_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationReviewViewDto, CommandError> {
    crate::observability::command::observe("get_translation_review_cmd", async {
        usecases
            .review
            .get(GetTranslationReviewRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                result_id: parse_translation_id(&result_id)?,
            })
            .await
            .map(Into::into)
            .map_err(CommandError::from)
    })
    .await
}

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
#[allow(clippy::too_many_arguments)] // Tauri binds stable top-level IPC fields and injected state separately.
pub async fn record_translation_review_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    result_id: String,
    expected_result_sha256: String,
    expected_link_revision: u64,
    decision: TranslationReviewDecisionDto,
    reviewer_label: String,
    note: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
    app: AppHandle,
) -> Result<TranslationReviewViewDto, CommandError> {
    crate::observability::command::observe("record_translation_review_cmd", async {
        let view = usecases
            .review
            .record(RecordTranslationReviewRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                result_id: parse_translation_id(&result_id)?,
                expected_result_sha256,
                expected_link_revision,
                decision: decision.into(),
                reviewer_label,
                note,
            })
            .await
            .map_err(CommandError::from)?;
        if adapters_tauri::TauriEventPublisher::new(app)
            .publish_project_updated(&project_id)
            .await
            .is_err()
        {
            tracing::warn!(
                code = "TranslationReviewNotificationFailed",
                "review decision will be recovered by snapshot refresh"
            );
        }
        Ok(view.into())
    })
    .await
}
