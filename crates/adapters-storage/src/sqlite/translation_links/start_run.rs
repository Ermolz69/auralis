use super::{row, validation};
use ports::error::PortError;
use ports::translation_links::{StartTranslationRun, TranslationLink};
use sqlx::SqlitePool;

pub(super) async fn run(
    pool: &SqlitePool,
    request: &StartTranslationRun,
) -> Result<TranslationLink, PortError> {
    validation::uuid(request.translation_id, "ID")?;
    validation::uuid(request.run_id, "run ID")?;
    if let Some(prior) = request.expected_active_run_id {
        validation::uuid(prior, "expected run ID")?;
    }
    if request.expected_link_revision == 0
        || request.expected_link_revision >= validation::MAX_LINK_REVISION
    {
        return Err(validation::conflict("invalid expected link revision"));
    }
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| super::super::helpers::map_sqlite_error("start_translation_run", error))?;
    let link = row::load(&mut *transaction, request.translation_id)
        .await?
        .ok_or_else(|| PortError::NotFound {
            resource: "translation link".into(),
        })?;
    if link.active_run_id == Some(request.run_id)
        && link.revision == request.expected_link_revision + 1
    {
        return Ok(link);
    }
    if link.revision != request.expected_link_revision
        || link.active_run_id != request.expected_active_run_id
        || link.active_run_id == Some(request.run_id)
    {
        return Err(validation::conflict("active run or link revision changed"));
    }
    let affected = sqlx::query(
        "UPDATE project_translations SET active_run_id = ?, revision = revision + 1, updated_at = ?
         WHERE translation_id = ? AND revision = ? AND active_run_id IS ?",
    )
    .bind(request.run_id.to_string())
    .bind(request.updated_at.to_rfc3339())
    .bind(request.translation_id.to_string())
    .bind(
        i64::try_from(request.expected_link_revision)
            .map_err(|_| validation::conflict("invalid expected link revision"))?,
    )
    .bind(request.expected_active_run_id.map(|id| id.to_string()))
    .execute(&mut *transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("start_translation_run", error))?;
    if affected.rows_affected() != 1 {
        return Err(validation::conflict("active run changed concurrently"));
    }
    let started = row::load(&mut *transaction, request.translation_id)
        .await?
        .ok_or_else(|| validation::conflict("translation link disappeared"))?;
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_translation_run", error)
    })?;
    Ok(started)
}
