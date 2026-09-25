use domain::outbox::{OutboxMessage, OutboxPayload};
use domain::project::ProjectId;
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

pub(super) async fn enqueue(
    transaction: &mut Transaction<'_, Sqlite>,
    project_id: &ProjectId,
) -> Result<(), PortError> {
    let raw_ids: Vec<String> =
        sqlx::query_scalar("SELECT translation_id FROM project_translations WHERE project_id = ? ORDER BY translation_id")
            .bind(project_id.to_string())
            .fetch_all(&mut **transaction)
            .await
            .map_err(|error| {
                crate::sqlite::helpers::map_sqlite_error("list_project_translations_for_cleanup", error)
            })?;
    for raw_id in raw_ids {
        let translation_id =
            Uuid::parse_str(&raw_id).map_err(|_| PortError::InvalidStoredData {
                entity_type: "project translation".into(),
                entity_id: raw_id,
                field: "translation_id".into(),
                message: "invalid translation ID".into(),
            })?;
        let message = OutboxMessage::new(OutboxPayload::DeleteProjectTranslation {
            project_id: project_id.clone(),
            translation_id,
        });
        super::outbox_writes::save_outbox_message(transaction, &message).await?;
    }
    Ok(())
}
