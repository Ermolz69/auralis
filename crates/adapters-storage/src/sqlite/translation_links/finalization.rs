use super::{automatic_finalization, manual_finalization, row, validation};
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
        super::super::helpers::map_sqlite_error("finalize_translation_publication", error)
    })?;
    let link = row::load(&mut *transaction, translation_id)
        .await?
        .ok_or_else(|| PortError::NotFound {
            resource: "translation link".into(),
        })?;
    let finalized = match manual_finalization::run(&mut transaction, &link, result_id, updated_at)
        .await?
    {
        Some(finalized) => finalized,
        None => automatic_finalization::run(&mut transaction, &link, result_id, updated_at).await?,
    };
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_publication_finalization", error)
    })?;
    Ok(finalized)
}
