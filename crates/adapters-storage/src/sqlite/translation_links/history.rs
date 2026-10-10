use super::{publication_row, row, validation};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::translation_links::{MAX_TRANSLATION_HISTORY_PAGE_SIZE, TranslationHistoryPage};
use sqlx::SqlitePool;
use uuid::Uuid;

pub(super) async fn list(
    pool: &SqlitePool,
    project_id: &ProjectId,
    translation_id: Uuid,
    before_result_id: Option<Uuid>,
    limit: u32,
) -> Result<TranslationHistoryPage, PortError> {
    if !(1..=MAX_TRANSLATION_HISTORY_PAGE_SIZE).contains(&limit) {
        return Err(validation::conflict(
            "invalid translation history page size",
        ));
    }
    let mut transaction = pool.begin().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("begin_translation_history", error)
    })?;
    let link = row::load(&mut *transaction, translation_id)
        .await?
        .filter(|link| &link.project_id == project_id)
        .ok_or_else(|| PortError::NotFound {
            resource: "project translation".into(),
        })?;
    let ready_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM translation_publications p JOIN artifacts a ON a.id = p.artifact_id
         WHERE p.translation_id = ? AND p.source_sha256 = ? AND p.state = 'ready'
           AND a.project_id = ? AND a.kind = 'TranslatedTranscript' AND a.state = 'ready'",
    )
    .bind(translation_id.to_string())
    .bind(&link.source_sha256)
    .bind(project_id.to_string())
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("count_translation_history", error))?;
    let ready_count = u64::try_from(ready_count)
        .map_err(|_| validation::conflict("invalid ready history count"))?;
    let before: Option<i64> = match before_result_id {
        Some(id) => Some(sqlx::query_scalar("SELECT rowid FROM translation_publications WHERE translation_id = ? AND result_id = ?")
            .bind(translation_id.to_string()).bind(id.to_string())
            .fetch_optional(&mut *transaction).await
            .map_err(|error| super::super::helpers::map_sqlite_error("read_translation_history_cursor", error))?
            .ok_or_else(|| validation::conflict("history cursor does not belong to this translation"))?),
        None => None,
    };
    let mut stored = sqlx::query_as::<_, publication_row::StoredPublication>(
        "SELECT p.translation_id, p.result_id, p.result_revision, p.run_id, p.artifact_id,
                p.source_sha256, p.expected_link_revision, p.needs_review, p.state
         FROM translation_publications p JOIN artifacts a ON a.id = p.artifact_id
         WHERE p.translation_id = ? AND p.source_sha256 = ? AND p.state = 'ready'
           AND a.project_id = ? AND a.kind = 'TranslatedTranscript' AND a.state = 'ready'
           AND (? IS NULL OR p.rowid < ?) ORDER BY p.rowid DESC LIMIT ?",
    )
    .bind(translation_id.to_string())
    .bind(&link.source_sha256)
    .bind(project_id.to_string())
    .bind(before)
    .bind(before)
    .bind(i64::from(limit) + 1)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("list_translation_history", error))?;
    let has_more = stored.len() > limit as usize;
    stored.truncate(limit as usize);
    let entries = stored
        .into_iter()
        .map(publication_row::StoredPublication::decode)
        .collect::<Result<Vec<_>, _>>()?;
    let next_cursor = if has_more {
        entries.last().map(|entry| entry.result_id)
    } else {
        None
    };
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("finish_translation_history", error)
    })?;
    Ok(TranslationHistoryPage {
        link,
        entries,
        ready_count,
        next_cursor,
    })
}
