#[allow(dead_code)]
#[path = "support/historical_control.rs"]
mod historical_control;
#[allow(dead_code)]
#[path = "support/historical_fixture.rs"]
mod historical_fixture;
#[allow(dead_code)]
#[path = "support/historical_provider.rs"]
mod historical_provider;
#[allow(dead_code)]
#[path = "support/historical_seed.rs"]
mod historical_seed;

use adapters_storage::sqlite::SqliteTranslationReviewStore;
use application::usecases::translation::{
    GetTranslationReviewRequest, RecordTranslationReviewRequest, TranslationReviewUseCase,
};
use domain::chrono::Utc;
use historical_fixture::HistoricalFixture;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{SelectTranslationHistoryResult, TranslationLinkStore};
use ports::translation_reviews::ReviewDecisionKind;
use std::{error::Error, sync::Arc};

#[tokio::test]
async fn srt_review_requires_exact_selected_revision() -> Result<(), Box<dyn Error>> {
    review_flow(TranslationSourceFormat::Srt).await
}

#[tokio::test]
async fn vtt_review_requires_exact_selected_revision() -> Result<(), Box<dyn Error>> {
    review_flow(TranslationSourceFormat::Vtt).await
}

async fn review_flow(format: TranslationSourceFormat) -> Result<(), Box<dyn Error>> {
    let mut fixture = HistoricalFixture::new(format).await?;
    let review = TranslationReviewUseCase::new(
        fixture.verifier(),
        Arc::new(SqliteTranslationReviewStore::new(fixture.pool.clone())),
    );
    let selected = review
        .get(GetTranslationReviewRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.head_id,
        })
        .await?;
    assert!(selected.selected);
    assert!(selected.latest.is_none());
    assert!(
        review
            .require_approved_selected(fixture.project_id.clone(), fixture.translation_id)
            .await
            .is_err()
    );
    assert!(
        review
            .record(RecordTranslationReviewRequest {
                project_id: fixture.project_id.clone(),
                translation_id: fixture.translation_id,
                result_id: fixture.head_id,
                expected_result_sha256: "f".repeat(64),
                expected_link_revision: selected.link_revision,
                decision: ReviewDecisionKind::Approved,
                reviewer_label: "Local reviewer".into(),
                note: "Compared both source cues".into(),
            })
            .await
            .is_err()
    );
    let approved = review
        .record(RecordTranslationReviewRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.head_id,
            expected_result_sha256: selected.result_sha256.clone(),
            expected_link_revision: selected.link_revision,
            decision: ReviewDecisionKind::Approved,
            reviewer_label: "Local reviewer".into(),
            note: "Compared both source cues".into(),
        })
        .await?;
    assert_eq!(
        approved.latest.as_ref().map(|event| event.decision),
        Some(ReviewDecisionKind::Approved)
    );
    assert_eq!(
        review
            .require_approved_selected(fixture.project_id.clone(), fixture.translation_id)
            .await?
            .result
            .result_id,
        fixture.head_id
    );

    let changed = fixture
        .links
        .select_historical_result(SelectTranslationHistoryResult {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.base_id,
            expected_link_revision: approved.link_revision,
            updated_at: Utc::now(),
        })
        .await?;
    assert_eq!(changed.selected_result_id, Some(fixture.base_id));
    assert!(
        review
            .require_approved_selected(fixture.project_id.clone(), fixture.translation_id)
            .await
            .is_err()
    );
    let older = review
        .get(GetTranslationReviewRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.head_id,
        })
        .await?;
    assert_eq!(
        older.latest.as_ref().map(|event| event.decision),
        Some(ReviewDecisionKind::Approved)
    );
    assert!(!older.selected);

    fixture.reopen().await?;
    let reopened = TranslationReviewUseCase::new(
        fixture.verifier(),
        Arc::new(SqliteTranslationReviewStore::new(fixture.pool.clone())),
    );
    assert!(
        reopened
            .require_approved_selected(fixture.project_id.clone(), fixture.translation_id)
            .await
            .is_err()
    );
    let retained = reopened
        .get(GetTranslationReviewRequest {
            project_id: fixture.project_id.clone(),
            translation_id: fixture.translation_id,
            result_id: fixture.head_id,
        })
        .await?;
    assert_eq!(
        retained.latest.as_ref().map(|event| event.decision),
        Some(ReviewDecisionKind::Approved)
    );
    Ok(())
}
