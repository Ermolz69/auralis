use super::{row, validation};
use ports::error::PortError;
use ports::translation_links::{TranslationIntent, TranslationLink};
use sqlx::SqlitePool;

pub(super) async fn run(
    pool: &SqlitePool,
    intent: &TranslationIntent,
) -> Result<TranslationLink, PortError> {
    validation::uuid(intent.translation_id, "ID")?;
    validation::uuid(intent.run_id, "run ID")?;
    validation::hash(&intent.source_sha256)?;
    validation::text(&intent.source_format, "source format")?;
    validation::text(&intent.source_language, "source language")?;
    validation::text(&intent.target_language, "target language")?;

    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| super::super::helpers::map_sqlite_error("begin_translation", error))?;
    if let Some(existing) = row::load(&mut *transaction, intent.translation_id).await? {
        if existing.project_id == intent.project_id
            && existing.source_artifact_id == intent.source_artifact_id
            && existing.source_sha256 == intent.source_sha256
            && existing.source_format == intent.source_format
            && existing.source_language == intent.source_language
            && existing.target_language == intent.target_language
            && existing.active_run_id == Some(intent.run_id)
        {
            return Ok(existing);
        }
        return Err(validation::conflict(
            "translation intent differs from stored link",
        ));
    }
    let source: Option<(String, String, String)> =
        sqlx::query_as("SELECT project_id, kind, state FROM artifacts WHERE id = ?")
            .bind(intent.source_artifact_id.to_string())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| {
                super::super::helpers::map_sqlite_error("check_translation_source", error)
            })?;
    let Some((project_id, kind, state)) = source else {
        return Err(PortError::NotFound {
            resource: "source subtitle artifact".into(),
        });
    };
    if project_id != intent.project_id.to_string() || kind != "OriginalSubtitle" || state != "ready"
    {
        return Err(validation::conflict(
            "source must be a ready original subtitle in the same project",
        ));
    }
    sqlx::query(
        "INSERT INTO project_translations (
            translation_id, project_id, source_artifact_id, source_sha256,
            source_format, source_language, target_language, active_run_id,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(intent.translation_id.to_string())
    .bind(intent.project_id.to_string())
    .bind(intent.source_artifact_id.to_string())
    .bind(&intent.source_sha256)
    .bind(&intent.source_format)
    .bind(&intent.source_language)
    .bind(&intent.target_language)
    .bind(intent.run_id.to_string())
    .bind(intent.created_at.to_rfc3339())
    .bind(intent.created_at.to_rfc3339())
    .execute(&mut *transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("insert_translation_link", error))?;
    let created = row::load(&mut *transaction, intent.translation_id)
        .await?
        .ok_or_else(|| validation::conflict("new translation link is missing"))?;
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_translation_link", error)
    })?;
    Ok(created)
}
