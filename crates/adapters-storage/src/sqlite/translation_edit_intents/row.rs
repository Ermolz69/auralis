use super::validation;
use ports::error::PortError;
use ports::translation_edit_intents::TranslationEditIntent;
use sqlx::{FromRow, SqliteExecutor};
use uuid::Uuid;

pub(super) const COLUMNS: &str = "i.project_id, i.translation_id, i.result_id, i.base_result_id, i.run_id, i.expected_head_result_id, i.expected_link_revision, i.request_sha256, i.created_at";

#[derive(FromRow)]
pub(super) struct EditIntentRow {
    project_id: String,
    translation_id: String,
    result_id: String,
    base_result_id: String,
    run_id: String,
    expected_head_result_id: String,
    expected_link_revision: i64,
    request_sha256: String,
    created_at: String,
}

impl EditIntentRow {
    pub(super) fn decode(self) -> Result<TranslationEditIntent, PortError> {
        let invalid = || PortError::InvalidStoredData {
            entity_type: "TranslationEditIntent".into(),
            entity_id: self.result_id.clone(),
            field: "metadata".into(),
            message: "invalid stored edit intent".into(),
        };
        let intent = TranslationEditIntent {
            project_id: self.project_id.parse().map_err(|_| invalid())?,
            translation_id: Uuid::parse_str(&self.translation_id).map_err(|_| invalid())?,
            result_id: Uuid::parse_str(&self.result_id).map_err(|_| invalid())?,
            base_result_id: Uuid::parse_str(&self.base_result_id).map_err(|_| invalid())?,
            run_id: Uuid::parse_str(&self.run_id).map_err(|_| invalid())?,
            expected_head_result_id: Uuid::parse_str(&self.expected_head_result_id)
                .map_err(|_| invalid())?,
            expected_link_revision: u64::try_from(self.expected_link_revision)
                .map_err(|_| invalid())?,
            request_sha256: self.request_sha256.clone(),
            created_at: self.created_at.parse().map_err(|_| invalid())?,
        };
        validation::validate(&intent).map_err(|_| invalid())?;
        Ok(intent)
    }
}

pub(crate) async fn load<'e, E: SqliteExecutor<'e>>(
    executor: E,
    result_id: Uuid,
) -> Result<Option<TranslationEditIntent>, PortError> {
    let sql = format!("SELECT {COLUMNS} FROM translation_edit_intents i WHERE i.result_id = ?");
    sqlx::query_as::<_, EditIntentRow>(sqlx::AssertSqlSafe(sql))
        .bind(result_id.to_string())
        .fetch_optional(executor)
        .await
        .map_err(|error| {
            super::super::helpers::map_sqlite_error("read_translation_edit_intent", error)
        })?
        .map(EditIntentRow::decode)
        .transpose()
}
