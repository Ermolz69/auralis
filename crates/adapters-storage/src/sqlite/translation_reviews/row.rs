use ports::error::PortError;
use ports::translation_reviews::{ReviewDecisionKind, TranslationReviewRecord};
use sqlx::{FromRow, SqliteExecutor};
use uuid::Uuid;

#[derive(FromRow)]
struct ReviewRow {
    event_id: String,
    project_id: String,
    translation_id: String,
    result_id: String,
    result_sha256: String,
    decision: String,
    reviewer_label: String,
    note: String,
    created_at: String,
}

impl ReviewRow {
    fn decode(self) -> Result<TranslationReviewRecord, PortError> {
        let invalid = || PortError::InvalidStoredData {
            entity_type: "TranslationReviewRecord".into(),
            entity_id: self.event_id.clone(),
            field: "metadata".into(),
            message: "invalid stored review event".into(),
        };
        let decision = match self.decision.as_str() {
            "approved" => ReviewDecisionKind::Approved,
            "withdrawn" => ReviewDecisionKind::Withdrawn,
            _ => return Err(invalid()),
        };
        Ok(TranslationReviewRecord {
            event_id: Uuid::parse_str(&self.event_id).map_err(|_| invalid())?,
            project_id: self.project_id.parse().map_err(|_| invalid())?,
            translation_id: Uuid::parse_str(&self.translation_id).map_err(|_| invalid())?,
            result_id: Uuid::parse_str(&self.result_id).map_err(|_| invalid())?,
            result_sha256: self.result_sha256,
            decision,
            reviewer_label: self.reviewer_label,
            note: self.note,
            created_at: self.created_at.parse().map_err(|_| invalid())?,
        })
    }
}

pub(super) async fn latest<'e, E: SqliteExecutor<'e>>(
    executor: E,
    project_id: &domain::project::ProjectId,
    translation_id: Uuid,
    result_id: Uuid,
) -> Result<Option<TranslationReviewRecord>, PortError> {
    sqlx::query_as::<_, ReviewRow>(
        "SELECT event_id, project_id, translation_id, result_id, result_sha256,
                decision, reviewer_label, note, created_at
         FROM translation_review_events
         WHERE project_id = ? AND translation_id = ? AND result_id = ?
         ORDER BY sequence DESC LIMIT 1",
    )
    .bind(project_id.to_string())
    .bind(translation_id.to_string())
    .bind(result_id.to_string())
    .fetch_optional(executor)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("read_translation_review", error))?
    .map(ReviewRow::decode)
    .transpose()
}
