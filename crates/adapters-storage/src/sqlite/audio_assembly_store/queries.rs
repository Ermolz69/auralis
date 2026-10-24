use super::{
    error::{database, invalid, uuid},
    mapper,
    row::AssemblyIntentRow,
    validation,
};
use crate::sqlite::{job_mapper::row_to_job, job_row::JobRow};
use domain::{job::Job, project::ProjectId};
use ports::{
    PortError,
    audio_jobs::{ActiveAssemblyAttempt, AudioAssemblyIntent, AudioAssemblyPublication},
};
use sqlx::{FromRow, Row, SqlitePool};
use uuid::Uuid;

pub(super) async fn get(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<AudioAssemblyIntent>, PortError> {
    sqlx::query_as::<_, AssemblyIntentRow>("SELECT assembly_id,project_id,source_run_id,source_attempt_id,plan_json,plan_sha256 FROM audio_assembly_intents WHERE assembly_id=?")
        .bind(id.to_string()).fetch_optional(pool).await.map_err(database)?.map(mapper::intent).transpose()
}

pub(super) async fn list_ids(
    pool: &SqlitePool,
    project: &ProjectId,
) -> Result<Vec<Uuid>, PortError> {
    let rows:Vec<String> = sqlx::query_scalar("SELECT assembly_id FROM audio_assembly_intents WHERE project_id=? ORDER BY created_at DESC,rowid DESC LIMIT 100")
        .bind(project.to_string()).fetch_all(pool).await.map_err(database)?;
    rows.iter().map(|id| uuid(id)).collect()
}

pub(super) async fn latest_job(pool: &SqlitePool, id: Uuid) -> Result<Option<Job>, PortError> {
    sqlx::query_as::<_, JobRow>("SELECT j.* FROM jobs j JOIN audio_assembly_attempts a ON a.job_id=j.id WHERE a.assembly_id=? ORDER BY a.created_at DESC,a.rowid DESC LIMIT 1")
        .bind(id.to_string()).fetch_optional(pool).await.map_err(database)?.map(row_to_job).transpose()
}

pub(super) async fn active(
    pool: &SqlitePool,
    id: Option<Uuid>,
) -> Result<Vec<ActiveAssemblyAttempt>, PortError> {
    let rows = sqlx::query("SELECT j.*,a.assembly_id AS assembly_identity,a.attempt_id AS attempt_identity FROM jobs j JOIN audio_assembly_attempts a ON a.job_id=j.id WHERE (a.ended_at IS NULL OR (? AND EXISTS(SELECT 1 FROM audio_assembly_publications p WHERE p.assembly_id=a.assembly_id AND p.attempt_id=a.attempt_id AND p.ready=0))) AND (? IS NULL OR a.assembly_id=?) ORDER BY a.created_at,a.rowid")
        .bind(id.is_none()).bind(id.map(|id| id.to_string())).bind(id.map(|id| id.to_string())).fetch_all(pool).await.map_err(database)?;
    rows.into_iter()
        .map(|row| {
            Ok(ActiveAssemblyAttempt {
                assembly_id: uuid(
                    &row.try_get::<String, _>("assembly_identity")
                        .map_err(database)?,
                )?,
                attempt_id: uuid(
                    &row.try_get::<String, _>("attempt_identity")
                        .map_err(database)?,
                )?,
                job: row_to_job(JobRow::from_row(&row).map_err(database)?)?,
            })
        })
        .collect()
}

pub(super) async fn publication(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<AudioAssemblyPublication>, PortError> {
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT publication_json,ready FROM audio_assembly_publications WHERE assembly_id=?",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(database)?;
    let value = row
        .map(|(json, ready)| {
            let mut value: AudioAssemblyPublication = serde_json::from_str(&json)
                .map_err(|_| invalid("Stored assembly publication JSON is invalid"))?;
            if value.assembly_id != id {
                return Err(invalid("Stored assembly publication identity differs"));
            }
            value.ready = ready;
            Ok(value)
        })
        .transpose()?;
    if let Some(value) = &value {
        let intent = get(pool, id)
            .await?
            .ok_or_else(|| invalid("Assembly publication has no intent"))?;
        let mut pending = value.clone();
        pending.ready = false;
        validation::publication(&pending, &validation::intent(&intent)?)?;
    }
    Ok(value)
}
