use super::{EditHistoricalTranslationRequest, VerifyTranslationHistoryUseCase, edit_fingerprint};
use crate::error::ApplicationError;
use ports::error::PortError;
use ports::translation_branch_edit::TranslationBranchEdit;
use ports::translation_edit_intents::{TranslationEditIntent, TranslationEditIntentStore};
use ports::translation_result_edit::{TranslationResultEdit, TranslationResultEditorPort};
use std::sync::Arc;
use uuid::Uuid;

pub struct EditHistoricalTranslationUseCase {
    verifier: Arc<VerifyTranslationHistoryUseCase>,
    intents: Arc<dyn TranslationEditIntentStore>,
    editor: Arc<dyn TranslationResultEditorPort>,
}

impl EditHistoricalTranslationUseCase {
    pub fn new(
        verifier: Arc<VerifyTranslationHistoryUseCase>,
        intents: Arc<dyn TranslationEditIntentStore>,
        editor: Arc<dyn TranslationResultEditorPort>,
    ) -> Self {
        Self {
            verifier,
            intents,
            editor,
        }
    }

    pub async fn execute(
        &self,
        request: EditHistoricalTranslationRequest,
    ) -> Result<Uuid, ApplicationError> {
        let base = self
            .verifier
            .execute(
                request.project_id.clone(),
                request.translation_id,
                request.base_result_id,
            )
            .await?;
        let mut intent = TranslationEditIntent {
            project_id: request.project_id,
            translation_id: request.translation_id,
            result_id: request.result_id,
            base_result_id: request.base_result_id,
            run_id: base.intent.run_id,
            expected_head_result_id: request.expected_head_result_id,
            expected_link_revision: request.expected_link_revision,
            request_sha256: String::new(),
            created_at: domain::chrono::Utc::now(),
        };
        intent.request_sha256 =
            edit_fingerprint::digest(&intent, request.segment_id, &request.lines);
        self.intents.record(intent).await?;
        let result_id = self
            .editor
            .commit_branch_edit(
                &base.source_path,
                &base.link,
                &base.intent,
                TranslationBranchEdit {
                    edit: TranslationResultEdit {
                        base_result_id: request.base_result_id,
                        result_id: request.result_id,
                        segment_id: request.segment_id,
                        lines: request.lines,
                    },
                    expected_head_result_id: request.expected_head_result_id,
                },
            )
            .await?;
        if result_id != request.result_id {
            return Err(PortError::Conflict {
                resource: "historical translation edit".into(),
                message: "Committed result differs from admitted request".into(),
            }
            .into());
        }
        #[cfg(feature = "native-e2e")]
        if request.base_result_id != request.expected_head_result_id {
            super::native_e2e_historical_edit_gap::hold_if_requested(base.intent.run_id, result_id)
                .await?;
        }
        Ok(result_id)
    }
}
