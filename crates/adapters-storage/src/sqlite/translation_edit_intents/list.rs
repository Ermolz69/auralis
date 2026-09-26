use super::{row, validation};
use ports::error::PortError;
use ports::translation_edit_intents::TranslationEditIntent;
use sqlx::SqlitePool;
use uuid::Uuid;

pub(super) async fn run(
    pool: &SqlitePool,
    after_result_id: Option<Uuid>,
    limit: u32,
) -> Result<Vec<TranslationEditIntent>, PortError> {
    if limit == 0 || limit > validation::MAX_RECOVERY_PAGE {
        return Err(PortError::InvalidSource {
            message: "invalid edit recovery page size".into(),
        });
    }
    if let Some(after) = after_result_id
        && row::load(pool, after).await?.is_none()
    {
        return Err(validation::conflict("edit recovery cursor does not exist"));
    }
    let sql = format!(
        "SELECT {} FROM translation_edit_intents i
         WHERE (? IS NULL OR (i.created_at, i.result_id) > (SELECT created_at, result_id FROM translation_edit_intents WHERE result_id = ?))
         AND NOT EXISTS (SELECT 1 FROM translation_publications p
             JOIN artifacts a ON a.id = p.artifact_id AND a.state = 'ready'
                 AND a.kind = 'TranslatedTranscript' AND a.project_id = i.project_id
             WHERE p.translation_id = i.translation_id AND p.result_id = i.result_id
                 AND p.run_id = i.run_id AND p.state = 'ready')
         ORDER BY i.created_at, i.result_id LIMIT ?", row::COLUMNS
    );
    let after = after_result_id.map(|id| id.to_string());
    sqlx::query_as::<_, row::EditIntentRow>(sqlx::AssertSqlSafe(sql))
        .bind(&after)
        .bind(&after)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|error| super::super::helpers::map_sqlite_error("list_unpublished_edits", error))?
        .into_iter()
        .map(row::EditIntentRow::decode)
        .collect()
}
