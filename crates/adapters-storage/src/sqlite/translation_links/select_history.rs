use super::{row, validation};
use ports::error::PortError;
use ports::translation_links::{SelectTranslationHistoryResult, TranslationLink};
use sqlx::SqlitePool;

pub(super) async fn run(
    pool: &SqlitePool,
    request: SelectTranslationHistoryResult,
) -> Result<TranslationLink, PortError> {
    validation::uuid(request.translation_id, "ID")?;
    validation::uuid(request.result_id, "result ID")?;
    if request.expected_link_revision == 0
        || request.expected_link_revision >= validation::MAX_LINK_REVISION
    {
        return Err(validation::conflict("invalid expected link revision"));
    }
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
        super::super::helpers::map_sqlite_error("begin_history_selection", error)
    })?;
    let link = row::load(&mut *transaction, request.translation_id)
        .await?
        .filter(|link| link.project_id == request.project_id)
        .ok_or_else(|| PortError::NotFound {
            resource: "project translation".into(),
        })?;
    let artifact_id: Option<String> = sqlx::query_scalar(
        "SELECT p.artifact_id FROM translation_publications p JOIN artifacts a ON a.id = p.artifact_id
         WHERE p.translation_id = ? AND p.result_id = ? AND p.source_sha256 = ? AND p.state = 'ready'
           AND a.project_id = ? AND a.kind = 'TranslatedTranscript' AND a.state = 'ready'",
    ).bind(request.translation_id.to_string()).bind(request.result_id.to_string())
        .bind(&link.source_sha256).bind(request.project_id.to_string())
        .fetch_optional(&mut *transaction).await
        .map_err(|error| super::super::helpers::map_sqlite_error("read_history_selection", error))?;
    let artifact_id = artifact_id
        .ok_or_else(|| validation::conflict("historical result is not a ready project artifact"))?;
    if link.selected_result_id == Some(request.result_id)
        && link
            .output_artifact_id
            .as_ref()
            .is_some_and(|id| id.to_string() == artifact_id)
    {
        return Ok(link);
    }
    if link.revision != request.expected_link_revision {
        return Err(validation::conflict(
            "project translation changed; refresh before choosing a version",
        ));
    }
    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM translation_publications WHERE translation_id = ? AND source_sha256 = ? AND expected_link_revision = ? AND state = 'pending')")
        .bind(request.translation_id.to_string()).bind(&link.source_sha256).bind(i64::try_from(link.revision).map_err(|_| validation::conflict("invalid link revision"))?)
        .fetch_one(&mut *transaction).await.map_err(|error| super::super::helpers::map_sqlite_error("check_pending_history_publication", error))?;
    if pending {
        return Err(PortError::Busy {
            message: "A new result is being published; wait before choosing a historical version"
                .into(),
        });
    }
    let updated = sqlx::query("UPDATE project_translations SET selected_result_id = ?, output_artifact_id = ?, revision = revision + 1, updated_at = ? WHERE translation_id = ? AND revision = ?")
        .bind(request.result_id.to_string()).bind(artifact_id).bind(request.updated_at.to_rfc3339())
        .bind(request.translation_id.to_string()).bind(i64::try_from(link.revision).map_err(|_| validation::conflict("invalid link revision"))?)
        .execute(&mut *transaction).await.map_err(|error| super::super::helpers::map_sqlite_error("select_history_result", error))?;
    if updated.rows_affected() != 1 {
        return Err(validation::conflict(
            "project translation changed during selection",
        ));
    }
    let selected = row::load(&mut *transaction, request.translation_id)
        .await?
        .ok_or_else(|| validation::conflict("selected link disappeared"))?;
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_history_selection", error)
    })?;
    Ok(selected)
}
