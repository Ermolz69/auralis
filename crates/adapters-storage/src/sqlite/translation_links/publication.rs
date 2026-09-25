use super::{row, validation};
use ports::error::PortError;
use ports::translation_links::PublicationIntent;
#[cfg(test)]
use sqlx::SqlitePool;
use sqlx::{Sqlite, Transaction};

#[cfg(test)]
pub(super) async fn record(pool: &SqlitePool, intent: &PublicationIntent) -> Result<(), PortError> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
        super::super::helpers::map_sqlite_error("record_translation_publication", error)
    })?;
    record_in_tx(&mut transaction, intent).await?;
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_translation_publication", error)
    })
}

pub(super) async fn record_in_tx(
    transaction: &mut Transaction<'_, Sqlite>,
    intent: &PublicationIntent,
) -> Result<(), PortError> {
    validation::uuid(intent.translation_id, "ID")?;
    validation::uuid(intent.result_id, "result ID")?;
    validation::uuid(intent.run_id, "run ID")?;
    validation::hash(&intent.source_sha256)?;
    if intent.result_revision == 0 || intent.result_revision > i32::MAX as u32 {
        return Err(validation::conflict("invalid result revision"));
    }
    if intent.expected_link_revision == 0
        || intent.expected_link_revision >= validation::MAX_LINK_REVISION
    {
        return Err(validation::conflict("invalid expected link revision"));
    }
    let expected_revision = i64::try_from(intent.expected_link_revision)
        .map_err(|_| validation::conflict("invalid expected link revision"))?;
    let existing: Option<(String, String, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT run_id, artifact_id, source_sha256, expected_link_revision, needs_review, result_revision
         FROM translation_publications WHERE translation_id = ? AND result_id = ?",
    )
    .bind(intent.translation_id.to_string())
    .bind(intent.result_id.to_string())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("read_translation_publication", error)
    })?;
    if let Some((run_id, artifact_id, source_sha256, revision, needs_review, result_revision)) =
        existing
    {
        if run_id == intent.run_id.to_string()
            && artifact_id == intent.artifact_id.to_string()
            && source_sha256 == intent.source_sha256
            && revision == expected_revision
            && needs_review == i64::from(intent.needs_review)
            && result_revision == i64::from(intent.result_revision)
        {
            return Ok(());
        }
        return Err(validation::conflict(
            "result publication differs from stored record",
        ));
    }
    let link = row::load(&mut **transaction, intent.translation_id)
        .await?
        .ok_or_else(|| PortError::NotFound {
            resource: "translation link".into(),
        })?;
    let selected_run_matches = if link.active_run_id.is_none() {
        if let Some(selected_result_id) = link.selected_result_id {
            let selected_run: Option<String> = sqlx::query_scalar(
                "SELECT run_id FROM translation_publications
                 WHERE translation_id = ? AND result_id = ? AND source_sha256 = ? AND state = 'ready'",
            )
            .bind(intent.translation_id.to_string())
            .bind(selected_result_id.to_string())
            .bind(&link.source_sha256)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|error| {
                super::super::helpers::map_sqlite_error("read_selected_translation_run", error)
            })?;
            selected_run.as_deref() == Some(intent.run_id.to_string().as_str())
        } else {
            false
        }
    } else {
        false
    };
    if !(link.active_run_id == Some(intent.run_id) || selected_run_matches)
        || link.source_sha256 != intent.source_sha256
        || link.revision != intent.expected_link_revision
    {
        return Err(validation::conflict(
            "run, source, or link revision changed",
        ));
    }
    let newer_exists: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM translation_publications
         WHERE translation_id = ? AND run_id = ? AND source_sha256 = ?
           AND expected_link_revision = ? AND result_revision > ?)",
    )
    .bind(intent.translation_id.to_string())
    .bind(intent.run_id.to_string())
    .bind(&intent.source_sha256)
    .bind(expected_revision)
    .bind(i64::from(intent.result_revision))
    .fetch_one(&mut **transaction)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("check_newer_translation_result", error)
    })?;
    if newer_exists != 0 {
        return Err(validation::conflict(
            "a newer result is already staged for publication",
        ));
    }
    let artifact: Option<(String, String, String)> =
        sqlx::query_as("SELECT project_id, kind, state FROM artifacts WHERE id = ?")
            .bind(intent.artifact_id.to_string())
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|error| {
                super::super::helpers::map_sqlite_error("check_translation_output", error)
            })?;
    let Some((project_id, kind, state)) = artifact else {
        return Err(PortError::NotFound {
            resource: "output artifact".into(),
        });
    };
    if project_id != link.project_id.to_string()
        || kind != "TranslatedTranscript"
        || !matches!(state.as_str(), "pending_finalize" | "ready")
    {
        return Err(validation::conflict(
            "output must be a pending or ready translated artifact in the same project",
        ));
    }
    sqlx::query(
        "INSERT INTO translation_publications (
            translation_id, result_id, run_id, artifact_id, source_sha256,
            expected_link_revision, state, needs_review, created_at, updated_at, result_revision
        ) VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?)",
    )
    .bind(intent.translation_id.to_string())
    .bind(intent.result_id.to_string())
    .bind(intent.run_id.to_string())
    .bind(intent.artifact_id.to_string())
    .bind(&intent.source_sha256)
    .bind(expected_revision)
    .bind(i64::from(intent.needs_review))
    .bind(intent.created_at.to_rfc3339())
    .bind(intent.created_at.to_rfc3339())
    .bind(i64::from(intent.result_revision))
    .execute(&mut **transaction)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("insert_translation_publication", error)
    })?;
    Ok(())
}
