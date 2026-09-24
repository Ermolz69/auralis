use super::{row, validation};
use chrono::{DateTime, Utc};
use ports::error::PortError;
use ports::translation_links::TranslationLink;
use sqlx::SqlitePool;
use uuid::Uuid;

pub(super) async fn run(
    pool: &SqlitePool,
    translation_id: Uuid,
    result_id: Uuid,
    updated_at: DateTime<Utc>,
) -> Result<TranslationLink, PortError> {
    validation::uuid(translation_id, "ID")?;
    validation::uuid(result_id, "result ID")?;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
        super::super::helpers::map_sqlite_error("select_translation_result", error)
    })?;
    let link = row::load(&mut *transaction, translation_id)
        .await?
        .ok_or_else(|| PortError::NotFound {
            resource: "translation link".into(),
        })?;
    let publication: Option<(String, String, String, i64, String)> = sqlx::query_as(
        "SELECT run_id, artifact_id, source_sha256, expected_link_revision, state
         FROM translation_publications WHERE translation_id = ? AND result_id = ?",
    )
    .bind(translation_id.to_string())
    .bind(result_id.to_string())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("read_translation_publication", error)
    })?;
    let Some((run_id, artifact_id, source_sha256, expected_revision, state)) = publication else {
        return Err(PortError::NotFound {
            resource: "translation publication".into(),
        });
    };
    if link.selected_result_id == Some(result_id)
        && link
            .output_artifact_id
            .as_ref()
            .is_some_and(|id| id.to_string() == artifact_id)
        && state == "ready"
    {
        return Ok(link);
    }
    if state != "pending" && state != "ready" {
        return Err(validation::conflict("publication is not pending or ready"));
    }
    if link.active_run_id.is_none_or(|id| id.to_string() != run_id)
        || link.source_sha256 != source_sha256
        || i64::try_from(link.revision).ok() != Some(expected_revision)
    {
        return Err(validation::conflict(
            "publication no longer matches active run and link revision",
        ));
    }
    let artifact: Option<(String, String, String)> =
        sqlx::query_as("SELECT project_id, kind, state FROM artifacts WHERE id = ?")
            .bind(&artifact_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| {
                super::super::helpers::map_sqlite_error("check_translation_output", error)
            })?;
    if artifact.as_ref().is_none_or(|(project_id, kind, state)| {
        project_id != &link.project_id.to_string()
            || kind != "TranslatedTranscript"
            || state != "ready"
    }) {
        return Err(validation::conflict(
            "output artifact is not ready in the project",
        ));
    }
    let changed = sqlx::query(
        "UPDATE project_translations
         SET selected_result_id = ?, output_artifact_id = ?, active_run_id = NULL,
             revision = revision + 1, updated_at = ?
         WHERE translation_id = ? AND revision = ? AND active_run_id = ? AND source_sha256 = ?",
    )
    .bind(result_id.to_string())
    .bind(&artifact_id)
    .bind(updated_at.to_rfc3339())
    .bind(translation_id.to_string())
    .bind(expected_revision)
    .bind(&run_id)
    .bind(&source_sha256)
    .execute(&mut *transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("select_translation_result", error))?;
    if changed.rows_affected() != 1 {
        return Err(validation::conflict(
            "translation selection changed concurrently",
        ));
    }
    sqlx::query(
        "UPDATE translation_publications SET state = 'ready', updated_at = ?
         WHERE translation_id = ? AND result_id = ?",
    )
    .bind(updated_at.to_rfc3339())
    .bind(translation_id.to_string())
    .bind(result_id.to_string())
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("finish_translation_publication", error)
    })?;
    let selected = row::load(&mut *transaction, translation_id)
        .await?
        .ok_or_else(|| validation::conflict("selected translation link is missing"))?;
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_translation_selection", error)
    })?;
    Ok(selected)
}
