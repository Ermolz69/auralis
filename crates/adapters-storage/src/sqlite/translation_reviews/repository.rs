use super::row;
use async_trait::async_trait;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::translation_reviews::{
    MAX_REVIEW_NOTE_LENGTH, MAX_REVIEWER_LABEL_LENGTH, RecordTranslationReview, ReviewDecisionKind,
    TranslationReviewRecord, TranslationReviewStore,
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteTranslationReviewStore {
    pool: SqlitePool,
}

impl SqliteTranslationReviewStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TranslationReviewStore for SqliteTranslationReviewStore {
    async fn selected_approval_matches(
        &self,
        project_id: &ProjectId,
        translation_id: Uuid,
        result_id: Uuid,
        result_sha256: &str,
        expected_link_revision: u64,
    ) -> Result<bool, PortError> {
        let revision =
            i64::try_from(expected_link_revision).map_err(|_| conflict("invalid link revision"))?;
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM project_translations l
             JOIN translation_publications p ON p.translation_id = l.translation_id AND p.result_id = l.selected_result_id
             JOIN artifacts a ON a.id = p.artifact_id AND a.project_id = l.project_id
             JOIN translation_review_events e ON e.project_id = l.project_id
                 AND e.translation_id = l.translation_id AND e.result_id = p.result_id
             WHERE l.project_id = ? AND l.translation_id = ? AND l.selected_result_id = ?
               AND l.revision = ? AND p.state = 'ready' AND p.source_sha256 = l.source_sha256
               AND a.state = 'ready' AND a.kind = 'TranslatedTranscript'
               AND e.sequence = (SELECT MAX(sequence) FROM translation_review_events
                                 WHERE project_id = l.project_id AND translation_id = l.translation_id
                                   AND result_id = p.result_id)
               AND e.decision = 'approved' AND e.result_sha256 = ?)",
        )
        .bind(project_id.to_string())
        .bind(translation_id.to_string())
        .bind(result_id.to_string())
        .bind(revision)
        .bind(result_sha256)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| super::super::helpers::map_sqlite_error("verify_selected_translation_approval", error))
    }

    async fn latest(
        &self,
        project_id: &ProjectId,
        translation_id: Uuid,
        result_id: Uuid,
    ) -> Result<Option<TranslationReviewRecord>, PortError> {
        row::latest(&self.pool, project_id, translation_id, result_id).await
    }

    async fn record(
        &self,
        request: RecordTranslationReview,
    ) -> Result<TranslationReviewRecord, PortError> {
        let event = &request.record;
        validate(event)?;
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| {
                super::super::helpers::map_sqlite_error("begin_translation_review", error)
            })?;
        let ready: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM project_translations l
             JOIN translation_publications p ON p.translation_id = l.translation_id AND p.result_id = ?
             JOIN artifacts a ON a.id = p.artifact_id AND a.project_id = l.project_id
             WHERE l.translation_id = ? AND l.project_id = ? AND l.revision = ?
               AND p.state = 'ready' AND p.source_sha256 = l.source_sha256
               AND a.state = 'ready' AND a.kind = 'TranslatedTranscript')",
        )
        .bind(event.result_id.to_string())
        .bind(event.translation_id.to_string())
        .bind(event.project_id.to_string())
        .bind(i64::try_from(request.expected_link_revision).map_err(|_| conflict("invalid link revision"))?)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| super::super::helpers::map_sqlite_error("verify_translation_review", error))?;
        if !ready {
            return Err(conflict(
                "publication, project or link revision changed before review",
            ));
        }
        if let Some(previous) = row::latest(
            &mut *transaction,
            &event.project_id,
            event.translation_id,
            event.result_id,
        )
        .await?
        {
            if previous.result_sha256 != event.result_sha256 {
                return Err(conflict("review digest differs from an earlier decision"));
            }
            if previous.decision == event.decision
                && previous.reviewer_label == event.reviewer_label
                && previous.note == event.note
            {
                return Ok(previous);
            }
        }
        let decision = match event.decision {
            ReviewDecisionKind::Approved => "approved",
            ReviewDecisionKind::Withdrawn => "withdrawn",
        };
        sqlx::query(
            "INSERT INTO translation_review_events
             (event_id, project_id, translation_id, result_id, result_sha256,
              decision, reviewer_label, note, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(event.event_id.to_string())
        .bind(event.project_id.to_string())
        .bind(event.translation_id.to_string())
        .bind(event.result_id.to_string())
        .bind(&event.result_sha256)
        .bind(decision)
        .bind(&event.reviewer_label)
        .bind(&event.note)
        .bind(event.created_at.to_rfc3339())
        .execute(&mut *transaction)
        .await
        .map_err(|error| {
            super::super::helpers::map_sqlite_error("record_translation_review", error)
        })?;
        transaction.commit().await.map_err(|error| {
            super::super::helpers::map_sqlite_error("commit_translation_review", error)
        })?;
        Ok(event.clone())
    }
}

fn validate(record: &TranslationReviewRecord) -> Result<(), PortError> {
    if record.result_sha256.len() != 64
        || !record
            .result_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("review digest must be lowercase SHA-256"));
    }
    if record.reviewer_label.trim().is_empty()
        || record.note.trim().is_empty()
        || record.reviewer_label.chars().count() > MAX_REVIEWER_LABEL_LENGTH
        || record.note.chars().count() > MAX_REVIEW_NOTE_LENGTH
    {
        return Err(invalid("reviewer label or note is invalid"));
    }
    Ok(())
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation review".into(),
        message: message.into(),
    }
}
