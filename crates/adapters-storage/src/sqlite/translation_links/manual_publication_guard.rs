use super::validation;
use ports::error::PortError;
use ports::translation_links::{PublicationIntent, TranslationLink};
use sqlx::{Sqlite, Transaction};

pub(super) async fn check(
    transaction: &mut Transaction<'_, Sqlite>,
    intent: &PublicationIntent,
    link: &TranslationLink,
) -> Result<bool, PortError> {
    let Some(edit) =
        super::super::translation_edit_intents::load_intent(&mut **transaction, intent.result_id)
            .await?
    else {
        return Ok(false);
    };
    if edit.project_id != link.project_id
        || edit.translation_id != intent.translation_id
        || edit.run_id != intent.run_id
        || edit.expected_link_revision != intent.expected_link_revision
        || intent.source_sha256 != link.source_sha256
        || !intent.needs_review
    {
        return Err(validation::conflict(
            "manual publication differs from its edit intent",
        ));
    }
    let base_revision: Option<i64> = sqlx::query_scalar(
        "SELECT p.result_revision FROM translation_publications p
         JOIN translation_run_intents r ON r.run_id = p.run_id AND r.translation_id = p.translation_id
         JOIN artifacts a ON a.id = p.artifact_id AND a.project_id = ?
            AND a.kind = 'TranslatedTranscript' AND a.state = 'ready'
         WHERE p.translation_id = ? AND p.result_id = ? AND p.run_id = ?
            AND p.source_sha256 = ? AND p.state = 'ready'"
    ).bind(link.project_id.to_string()).bind(edit.translation_id.to_string())
        .bind(edit.base_result_id.to_string()).bind(edit.run_id.to_string())
        .bind(&link.source_sha256).fetch_optional(&mut **transaction).await
        .map_err(|error| super::super::helpers::map_sqlite_error("verify_manual_publication_base", error))?;
    if base_revision.is_none_or(|base| i64::from(intent.result_revision) <= base) {
        return Err(validation::conflict(
            "manual publication has no ready base or later revision",
        ));
    }
    Ok(true)
}
