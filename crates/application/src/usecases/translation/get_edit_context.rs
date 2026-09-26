use super::{
    GetHistoricalTranslationEditContextRequest, HistoricalTranslationEditContext,
    VerifyTranslationHistoryUseCase,
};
use crate::error::ApplicationError;
use ports::error::PortError;
use ports::translation_result::TranslationResultPort;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::sync::Arc;

pub struct GetHistoricalTranslationEditContextUseCase {
    verifier: Arc<VerifyTranslationHistoryUseCase>,
    control: Arc<dyn TranslationRunControlPort>,
    results: Arc<dyn TranslationResultPort>,
}

impl GetHistoricalTranslationEditContextUseCase {
    pub fn new(
        verifier: Arc<VerifyTranslationHistoryUseCase>,
        control: Arc<dyn TranslationRunControlPort>,
        results: Arc<dyn TranslationResultPort>,
    ) -> Self {
        Self {
            verifier,
            control,
            results,
        }
    }
    pub async fn execute(
        &self,
        request: GetHistoricalTranslationEditContextRequest,
    ) -> Result<HistoricalTranslationEditContext, ApplicationError> {
        let base = self
            .verifier
            .execute(
                request.project_id.clone(),
                request.translation_id,
                request.base_result_id,
            )
            .await?;
        let snapshot = self
            .control
            .snapshot_for_run(&base.link, base.intent.run_id)
            .await?;
        if snapshot.run_id != base.intent.run_id || snapshot.phase != TranslationRunPhase::Validated
        {
            return Err(conflict("Historical run has no validated observed head"));
        }
        let head_id = snapshot
            .result_id
            .ok_or_else(|| conflict("Validated historical run has no head result"))?;
        let head = self
            .results
            .verified_frozen_result(&base.source_path, &base.link, &base.intent, head_id)
            .await?;
        if head.result_id != head_id
            || head.run_id != base.intent.run_id
            || head.result_revision < base.result.result_revision
        {
            return Err(conflict(
                "Observed head differs from the historical base run",
            ));
        }
        Ok(HistoricalTranslationEditContext {
            project_id: request.project_id,
            translation_id: request.translation_id,
            base_result_id: request.base_result_id,
            run_id: base.intent.run_id,
            base_revision: base.result.result_revision,
            expected_head_result_id: head_id,
            head_revision: head.result_revision,
            expected_link_revision: base.link.revision,
        })
    }
}

fn conflict(message: &str) -> ApplicationError {
    PortError::Conflict {
        resource: "historical edit context".into(),
        message: message.into(),
    }
    .into()
}
