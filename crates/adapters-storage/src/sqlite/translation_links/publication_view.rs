use super::{publication_row, row, validation};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::translation_links::{TranslationPublicationState, TranslationPublicationView};
use sqlx::SqlitePool;
use uuid::Uuid;

pub(super) async fn read(
    pool: &SqlitePool,
    project_id: &ProjectId,
    translation_id: Uuid,
    result_id: Uuid,
) -> Result<TranslationPublicationView, PortError> {
    validation::uuid(translation_id, "translation ID")?;
    validation::uuid(result_id, "result ID")?;
    let mut transaction = pool.begin().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("begin_publication_view", error)
    })?;
    let link = row::load(&mut *transaction, translation_id)
        .await?
        .filter(|link| &link.project_id == project_id)
        .ok_or_else(|| PortError::NotFound {
            resource: "project translation".into(),
        })?;
    let publication = publication_row::load(&mut *transaction, translation_id, result_id)
        .await?
        .filter(|publication| publication.source_sha256 == link.source_sha256)
        .ok_or_else(|| PortError::NotFound {
            resource: "project translation publication".into(),
        })?;
    let artifact_state: Option<String> = sqlx::query_scalar("SELECT state FROM artifacts WHERE id = ? AND project_id = ? AND kind = 'TranslatedTranscript' AND location_kind = 'StorageKey'")
        .bind(publication.artifact_id.to_string()).bind(project_id.to_string()).fetch_optional(&mut *transaction).await
        .map_err(|error| super::super::helpers::map_sqlite_error("read_publication_artifact_state", error))?;
    if artifact_state.is_none()
        || publication.state == TranslationPublicationState::Ready
            && artifact_state.as_deref() != Some("ready")
        || link.selected_result_id == Some(result_id)
            && (publication.state != TranslationPublicationState::Ready
                || link.output_artifact_id.as_ref() != Some(&publication.artifact_id))
    {
        return Err(validation::conflict(
            "publication has no matching managed artifact state",
        ));
    }
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("finish_publication_view", error)
    })?;
    Ok(TranslationPublicationView { link, publication })
}
