use ports::error::PortError;
use ports::translation_links::ReadyPublication;
use sqlx::SqlitePool;
use uuid::Uuid;

pub(super) async fn list(
    pool: &SqlitePool,
    limit: usize,
) -> Result<Vec<ReadyPublication>, PortError> {
    let limit = i64::try_from(limit).map_err(|_| PortError::Conflict {
        resource: "translation publication".into(),
        message: "ready publication query limit is too large".into(),
    })?;
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT p.translation_id, p.result_id
         FROM translation_publications p
         JOIN artifacts a ON a.id = p.artifact_id
         JOIN project_translations l ON l.translation_id = p.translation_id
         WHERE p.state = 'pending' AND a.state = 'ready'
           AND a.kind = 'TranslatedTranscript' AND a.project_id = l.project_id
           AND (l.active_run_id = p.run_id OR
                (l.active_run_id IS NULL AND EXISTS (
                    SELECT 1 FROM translation_publications selected
                    WHERE selected.translation_id = l.translation_id
                      AND selected.result_id = l.selected_result_id
                      AND selected.run_id = p.run_id
                      AND selected.source_sha256 = l.source_sha256
                      AND selected.state = 'ready'
                )))
           AND l.source_sha256 = p.source_sha256
           AND l.revision = p.expected_link_revision
         ORDER BY p.created_at, p.result_id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("list_ready_translation_publications", error)
    })?;
    rows.into_iter()
        .map(|(translation_id, result_id)| {
            let invalid = |field: &str| PortError::InvalidStoredData {
                entity_type: "TranslationPublication".into(),
                entity_id: format!("{translation_id}:{result_id}"),
                field: field.into(),
                message: "invalid stored UUID".into(),
            };
            Ok(ReadyPublication {
                translation_id: Uuid::parse_str(&translation_id)
                    .map_err(|_| invalid("translation_id"))?,
                result_id: Uuid::parse_str(&result_id).map_err(|_| invalid("result_id"))?,
            })
        })
        .collect()
}
