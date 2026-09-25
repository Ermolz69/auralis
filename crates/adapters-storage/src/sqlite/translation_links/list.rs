use super::row::LinkRow;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::translation_links::TranslationLink;
use sqlx::SqlitePool;

const PROJECT_LINKS: &str = "SELECT translation_id, project_id, source_artifact_id,
    source_sha256, source_format, source_language, target_language, active_run_id,
    selected_result_id, output_artifact_id, revision, created_at, updated_at
    FROM project_translations WHERE project_id = ?
    ORDER BY created_at, translation_id";

const ACTIVE_LINKS: &str = "SELECT translation_id, project_id, source_artifact_id,
    source_sha256, source_format, source_language, target_language, active_run_id,
    selected_result_id, output_artifact_id, revision, created_at, updated_at
    FROM project_translations WHERE active_run_id IS NOT NULL
    ORDER BY updated_at, translation_id";

const RECOVERABLE_LINKS: &str = "SELECT translation_id, project_id, source_artifact_id,
    source_sha256, source_format, source_language, target_language, active_run_id,
    selected_result_id, output_artifact_id, revision, created_at, updated_at
    FROM project_translations WHERE active_run_id IS NOT NULL OR selected_result_id IS NOT NULL
    ORDER BY updated_at, translation_id";

pub(super) async fn by_project(
    pool: &SqlitePool,
    project_id: &ProjectId,
) -> Result<Vec<TranslationLink>, PortError> {
    let rows = sqlx::query_as::<_, LinkRow>(PROJECT_LINKS)
        .bind(project_id.to_string())
        .fetch_all(pool)
        .await
        .map_err(|error| {
            super::super::helpers::map_sqlite_error("list_project_translations", error)
        })?;
    rows.into_iter().map(LinkRow::decode).collect()
}

pub(super) async fn active(pool: &SqlitePool) -> Result<Vec<TranslationLink>, PortError> {
    let rows = sqlx::query_as::<_, LinkRow>(ACTIVE_LINKS)
        .fetch_all(pool)
        .await
        .map_err(|error| {
            super::super::helpers::map_sqlite_error("list_active_translations", error)
        })?;
    rows.into_iter().map(LinkRow::decode).collect()
}

pub(super) async fn recoverable(pool: &SqlitePool) -> Result<Vec<TranslationLink>, PortError> {
    let rows = sqlx::query_as::<_, LinkRow>(RECOVERABLE_LINKS)
        .fetch_all(pool)
        .await
        .map_err(|error| {
            super::super::helpers::map_sqlite_error("list_recoverable_translations", error)
        })?;
    rows.into_iter().map(LinkRow::decode).collect()
}
