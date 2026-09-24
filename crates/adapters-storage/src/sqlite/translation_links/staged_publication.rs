use super::{publication, row, validation};
use crate::sqlite::unit_of_work::insert_new_artifact;
use crate::sqlite::unit_of_work::outbox_writes::save_outbox_message;
use domain::media::{ArtifactKind, ArtifactState};
use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::error::PortError;
use ports::storage::StagedArtifact;
use ports::transaction::CommitStagedArtifactWrite;
use ports::translation_links::PublicationIntent;
use sqlx::{Sqlite, SqlitePool, Transaction};

pub(super) async fn commit(
    pool: &SqlitePool,
    intent: &PublicationIntent,
    staged: &StagedArtifact,
) -> Result<(), PortError> {
    if staged.artifact.id != intent.artifact_id
        || staged.artifact.kind != ArtifactKind::TranslatedTranscript
        || staged.artifact.state != ArtifactState::PendingFinalize
        || staged.artifact.size_bytes != Some(staged.size_bytes)
    {
        return Err(validation::conflict(
            "staged output does not match publication",
        ));
    }
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_staged_translation_publication", error)
    })?;
    let link = row::load(&mut *transaction, intent.translation_id)
        .await?
        .ok_or_else(|| PortError::NotFound {
            resource: "translation link".into(),
        })?;
    CommitStagedArtifactWrite {
        project_id: link.project_id.clone(),
        artifact: staged.artifact.clone(),
        staging_key: staged.staging_key.clone(),
        final_key: staged.final_key.clone(),
        temp_workspace_key: None,
    }
    .validate()?;

    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT state FROM translation_publications WHERE translation_id = ? AND result_id = ?",
    )
    .bind(intent.translation_id.to_string())
    .bind(intent.result_id.to_string())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("read_translation_publication", error)
    })?;
    if let Some((state,)) = existing {
        if !matches!(state.as_str(), "pending" | "ready") {
            return Err(validation::conflict(
                "publication cannot be retried in this state",
            ));
        }
        publication::record_in_tx(&mut transaction, intent).await?;
        verify_existing(&mut transaction, intent, staged, &link.project_id).await?;
    } else {
        insert_new_artifact(&mut transaction, &link.project_id, &staged.artifact).await?;
        publication::record_in_tx(&mut transaction, intent).await?;
        let mut message = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            project_id: link.project_id,
            artifact_id: staged.artifact.id.clone(),
            staging_key: staged.staging_key.clone(),
            final_key: staged.final_key.clone(),
        });
        message.deduplication_key = Some(deduplication_key(intent));
        save_outbox_message(&mut transaction, &message).await?;
    }
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_translation_publication", error)
    })
}

async fn verify_existing(
    transaction: &mut Transaction<'_, Sqlite>,
    intent: &PublicationIntent,
    staged: &StagedArtifact,
    project_id: &domain::project::ProjectId,
) -> Result<(), PortError> {
    let artifact: Option<(String, String, String, String, Option<i64>, String)> = sqlx::query_as(
        "SELECT project_id, kind, location_kind, location_value, size_bytes, state FROM artifacts WHERE id = ?",
    )
    .bind(intent.artifact_id.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("read_translation_output", error))?;
    let expected_size = i64::try_from(staged.size_bytes)
        .map_err(|_| validation::conflict("staged output size is invalid"))?;
    let Some((stored_project, kind, location_kind, location_value, size, state)) = artifact else {
        return Err(validation::conflict("published output artifact is missing"));
    };
    if stored_project != project_id.to_string()
        || kind != "TranslatedTranscript"
        || location_kind != "StorageKey"
        || location_value != staged.final_key
        || size != Some(expected_size)
        || !matches!(state.as_str(), "pending_finalize" | "ready")
    {
        return Err(validation::conflict(
            "published output artifact differs from retry",
        ));
    }
    if state == "pending_finalize" {
        let message: Option<(String, String)> = sqlx::query_as(
            "SELECT payload_json, status FROM outbox_messages WHERE deduplication_key = ?",
        )
        .bind(deduplication_key(intent))
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|error| {
            super::super::helpers::map_sqlite_error("read_translation_outbox", error)
        })?;
        let Some((payload, outbox_state)) = message else {
            return Err(validation::conflict(
                "pending publication has no finalization outbox",
            ));
        };
        if outbox_state == "dead" {
            return Err(validation::conflict(
                "pending publication finalization is dead",
            ));
        }
        let payload: OutboxPayload = serde_json::from_str(&payload).map_err(|_| {
            validation::conflict("pending publication has an invalid finalization outbox")
        })?;
        if !matches!(payload, OutboxPayload::FinalizeStagedArtifact {
            project_id: ref saved_project,
            artifact_id: ref saved_artifact,
            staging_key: ref saved_staging,
            final_key: ref saved_final,
        } if saved_project == project_id
            && saved_artifact == &intent.artifact_id
            && saved_staging == &staged.staging_key
            && saved_final == &staged.final_key)
        {
            return Err(validation::conflict(
                "pending publication outbox differs from retry",
            ));
        }
    }
    Ok(())
}

fn deduplication_key(intent: &PublicationIntent) -> String {
    format!(
        "translation-publication:{}:{}",
        intent.translation_id, intent.result_id
    )
}
