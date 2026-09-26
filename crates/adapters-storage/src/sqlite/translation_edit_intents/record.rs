use super::{row, validation};
use ports::error::PortError;
use ports::translation_edit_intents::TranslationEditIntent;
use sqlx::SqlitePool;

pub(super) async fn run(
    pool: &SqlitePool,
    intent: &TranslationEditIntent,
) -> Result<(), PortError> {
    validation::validate(intent)?;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
        super::super::helpers::map_sqlite_error("begin_translation_edit_intent", error)
    })?;
    let owner: Option<(String, i64)> = sqlx::query_as(
        "SELECT project_id, revision FROM project_translations WHERE translation_id = ?",
    )
    .bind(intent.translation_id.to_string())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("edit_intent_owner", error))?;
    let (project, revision) = owner.ok_or_else(|| PortError::NotFound {
        resource: "translation link".into(),
    })?;
    if project != intent.project_id.to_string() {
        return Err(validation::conflict("edit belongs to another project"));
    }
    if let Some(stored) = row::load(&mut *transaction, intent.result_id).await? {
        let mut expected = intent.clone();
        expected.created_at = stored.created_at;
        if expected != stored {
            return Err(validation::conflict(
                "result ID has a different edit request",
            ));
        }
        return Ok(());
    }
    if u64::try_from(revision).ok() != Some(intent.expected_link_revision) {
        return Err(validation::conflict(
            "project selection changed before edit admission",
        ));
    }
    let valid_base: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM translation_publications p
         JOIN translation_run_intents r ON r.run_id = p.run_id AND r.translation_id = p.translation_id
         JOIN artifacts a ON a.id = p.artifact_id AND a.project_id = ? AND a.state = 'ready' AND a.kind = 'TranslatedTranscript'
         WHERE p.translation_id = ? AND p.result_id = ? AND p.run_id = ? AND p.state = 'ready'
         AND p.source_sha256 = (SELECT source_sha256 FROM project_translations WHERE translation_id = p.translation_id))")
        .bind(project).bind(intent.translation_id.to_string()).bind(intent.base_result_id.to_string())
        .bind(intent.run_id.to_string()).fetch_one(&mut *transaction).await
        .map_err(|error| super::super::helpers::map_sqlite_error("verify_edit_intent_base", error))?;
    if !valid_base {
        return Err(validation::conflict(
            "edit base has no ready owned publication and frozen run",
        ));
    }
    let collision: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM translation_publications WHERE result_id = ?)",
    )
    .bind(intent.result_id.to_string())
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("verify_edit_result_id", error))?;
    if collision {
        return Err(validation::conflict(
            "new edit result already has a publication",
        ));
    }
    sqlx::query("INSERT INTO translation_edit_intents (project_id, translation_id, result_id, base_result_id, run_id, expected_head_result_id, expected_link_revision, request_sha256, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(intent.project_id.to_string()).bind(intent.translation_id.to_string())
        .bind(intent.result_id.to_string()).bind(intent.base_result_id.to_string())
        .bind(intent.run_id.to_string()).bind(intent.expected_head_result_id.to_string())
        .bind(i64::try_from(intent.expected_link_revision).map_err(|_| validation::conflict("invalid link revision"))?)
        .bind(&intent.request_sha256).bind(intent.created_at.to_rfc3339())
        .execute(&mut *transaction).await
        .map_err(|error| super::super::helpers::map_sqlite_error("record_translation_edit_intent", error))?;
    transaction.commit().await.map_err(|error| {
        super::super::helpers::map_sqlite_error("commit_translation_edit_intent", error)
    })
}
