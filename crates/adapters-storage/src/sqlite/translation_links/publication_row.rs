use domain::media::ArtifactId;
use ports::error::PortError;
use ports::translation_links::{TranslationPublicationRecord, TranslationPublicationState};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(FromRow)]
struct StoredPublication {
    translation_id: String,
    result_id: String,
    result_revision: i64,
    run_id: String,
    artifact_id: String,
    source_sha256: String,
    expected_link_revision: i64,
    needs_review: i64,
    state: String,
}

pub(super) async fn load(
    pool: &SqlitePool,
    translation_id: Uuid,
    result_id: Uuid,
) -> Result<Option<TranslationPublicationRecord>, PortError> {
    sqlx::query_as::<_, StoredPublication>(
        "SELECT translation_id, result_id, result_revision, run_id, artifact_id, source_sha256,
            expected_link_revision, needs_review, state
         FROM translation_publications WHERE translation_id = ? AND result_id = ?",
    )
    .bind(translation_id.to_string())
    .bind(result_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("read_translation_publication", error)
    })?
    .map(StoredPublication::decode)
    .transpose()
}

pub(super) async fn exists_for_run(
    pool: &SqlitePool,
    translation_id: Uuid,
    run_id: Uuid,
) -> Result<bool, PortError> {
    let exists: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM translation_publications WHERE translation_id = ? AND run_id = ?)",
    )
    .bind(translation_id.to_string())
    .bind(run_id.to_string())
    .fetch_one(pool)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("check_translation_run_publication", error)
    })?;
    Ok(exists != 0)
}

impl StoredPublication {
    fn decode(self) -> Result<TranslationPublicationRecord, PortError> {
        let invalid = |field: &str| PortError::InvalidStoredData {
            entity_type: "TranslationPublication".into(),
            entity_id: format!("{}:{}", self.translation_id, self.result_id),
            field: field.into(),
            message: "invalid stored value".into(),
        };
        let state = match self.state.as_str() {
            "pending" => TranslationPublicationState::Pending,
            "ready" => TranslationPublicationState::Ready,
            "failed" => TranslationPublicationState::Failed,
            _ => return Err(invalid("state")),
        };
        let needs_review = match self.needs_review {
            0 => false,
            1 => true,
            _ => return Err(invalid("needs_review")),
        };
        if self.source_sha256.len() != 64
            || !self
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid("source_sha256"));
        }
        Ok(TranslationPublicationRecord {
            translation_id: Uuid::parse_str(&self.translation_id)
                .map_err(|_| invalid("translation_id"))?,
            result_id: Uuid::parse_str(&self.result_id).map_err(|_| invalid("result_id"))?,
            result_revision: u32::try_from(self.result_revision)
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| invalid("result_revision"))?,
            run_id: Uuid::parse_str(&self.run_id).map_err(|_| invalid("run_id"))?,
            artifact_id: ArtifactId(
                Uuid::parse_str(&self.artifact_id).map_err(|_| invalid("artifact_id"))?,
            ),
            source_sha256: self.source_sha256,
            expected_link_revision: u64::try_from(self.expected_link_revision)
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| invalid("expected_link_revision"))?,
            needs_review,
            state,
        })
    }
}
