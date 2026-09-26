use super::{manual_publication_guard, publication_row, row, validation};
use chrono::{DateTime, Utc};
use ports::error::PortError;
use ports::translation_links::{PublicationIntent, TranslationLink, TranslationPublicationState};
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

pub(super) async fn run(
    transaction: &mut Transaction<'_, Sqlite>,
    link: &TranslationLink,
    result_id: Uuid,
    updated_at: DateTime<Utc>,
) -> Result<Option<TranslationLink>, PortError> {
    let Some(intent) =
        super::super::translation_edit_intents::load_intent(&mut **transaction, result_id).await?
    else {
        return Ok(None);
    };
    if intent.project_id != link.project_id || intent.translation_id != link.translation_id {
        return Err(validation::conflict(
            "manual finalization belongs to another project",
        ));
    }
    let Some(publication) =
        publication_row::load(&mut **transaction, link.translation_id, result_id).await?
    else {
        return Err(PortError::NotFound {
            resource: "manual translation publication".into(),
        });
    };
    if !matches!(
        publication.state,
        TranslationPublicationState::Pending | TranslationPublicationState::Ready
    ) {
        return Err(validation::conflict(
            "manual publication differs from edit intent or source",
        ));
    }
    manual_publication_guard::check(
        transaction,
        &PublicationIntent {
            translation_id: publication.translation_id,
            result_id: publication.result_id,
            result_revision: publication.result_revision,
            run_id: publication.run_id,
            artifact_id: publication.artifact_id.clone(),
            source_sha256: publication.source_sha256,
            expected_link_revision: publication.expected_link_revision,
            needs_review: publication.needs_review,
            created_at: updated_at,
        },
        link,
    )
    .await?;
    let artifact = publication.artifact_id.to_string();
    let ready: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ? AND project_id = ?
         AND kind = 'TranslatedTranscript' AND location_kind = 'StorageKey' AND state = 'ready')",
    )
    .bind(&artifact)
    .bind(link.project_id.to_string())
    .fetch_one(&mut **transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("check_manual_output", error))?;
    if !ready {
        return Err(validation::conflict("manual output artifact is not ready"));
    }
    if publication.state == TranslationPublicationState::Ready {
        return Ok(Some(link.clone()));
    }
    let automatic_pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM translation_publications p
         WHERE p.translation_id = ? AND p.expected_link_revision = ? AND p.state = 'pending'
         AND NOT EXISTS(SELECT 1 FROM translation_edit_intents i WHERE i.result_id = p.result_id))",
    )
    .bind(link.translation_id.to_string())
    .bind(
        i64::try_from(link.revision)
            .map_err(|_| validation::conflict("invalid current link revision"))?,
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("check_manual_attachment_order", error)
    })?;
    if link.revision == intent.expected_link_revision && !automatic_pending {
        let changed = sqlx::query(
            "UPDATE project_translations SET selected_result_id = ?, output_artifact_id = ?,
             revision = revision + 1, updated_at = ? WHERE translation_id = ? AND revision = ?",
        )
        .bind(result_id.to_string())
        .bind(&artifact)
        .bind(updated_at.to_rfc3339())
        .bind(link.translation_id.to_string())
        .bind(
            i64::try_from(intent.expected_link_revision)
                .map_err(|_| validation::conflict("invalid expected link revision"))?,
        )
        .execute(&mut **transaction)
        .await
        .map_err(|error| {
            super::super::helpers::map_sqlite_error("attach_manual_translation", error)
        })?;
        if changed.rows_affected() != 1 {
            return Err(validation::conflict(
                "manual attachment link changed concurrently",
            ));
        }
    }
    sqlx::query("UPDATE translation_publications SET state = 'ready', updated_at = ? WHERE translation_id = ? AND result_id = ? AND state = 'pending'")
        .bind(updated_at.to_rfc3339()).bind(link.translation_id.to_string()).bind(result_id.to_string())
        .execute(&mut **transaction).await
        .map_err(|error| super::super::helpers::map_sqlite_error("finish_manual_publication", error))?;
    let current = row::load(&mut **transaction, link.translation_id)
        .await?
        .ok_or_else(|| validation::conflict("manual translation link disappeared"))?;
    Ok(Some(current))
}
