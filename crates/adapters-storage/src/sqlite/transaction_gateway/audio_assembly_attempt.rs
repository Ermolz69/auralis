use crate::sqlite::{
    audio_assembly_store::error::{conflict, database, integer, invalid},
    job_mapper::job_to_row_values,
    unit_of_work::job_writes,
};
use domain::job::{Job, JobKind, JobStatus};
use ports::PortError;
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

pub(crate) async fn create_assembly_attempt(
    pool: &SqlitePool,
    id: Uuid,
    attempt: Uuid,
    job: Job,
) -> Result<Job, PortError> {
    if job.kind() != &JobKind::AudioAssembly
        || job.status() != &JobStatus::Pending
        || job.revision() != 1
        || job.stage().is_some()
        || job.started_at().is_some()
        || job.finished_at().is_some()
    {
        return Err(invalid(
            "Assembly attempt requires a new pending audioAssembly job",
        ));
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(database)?;
    let eligible:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audio_assembly_intents i WHERE i.assembly_id=? AND i.project_id=? AND NOT EXISTS(SELECT 1 FROM audio_assembly_publications p WHERE p.assembly_id=i.assembly_id))")
        .bind(id.to_string()).bind(job.project_id().to_string()).fetch_one(&mut *tx).await.map_err(database)?;
    if !eligible {
        return Err(conflict(
            "Assembly is missing, belongs to another project or already has a publication",
        ));
    }
    job_writes::insert_job(&mut tx, &job).await?;
    sqlx::query("INSERT INTO audio_assembly_attempts(attempt_id,assembly_id,job_id,created_at) VALUES (?,?,?,?)")
        .bind(attempt.to_string()).bind(id.to_string()).bind(job.id().to_string()).bind(chrono::Utc::now().to_rfc3339()).execute(&mut *tx).await.map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(job)
}

pub(crate) async fn update_assembly_attempt(
    pool: &SqlitePool,
    job: &Job,
    attempt: Uuid,
    expected: u64,
) -> Result<(), PortError> {
    if job.kind() != &JobKind::AudioAssembly
        || job.revision()
            != expected
                .checked_add(1)
                .ok_or_else(|| invalid("Assembly revision overflow"))?
        || !matches!(
            job.status(),
            JobStatus::Running | JobStatus::Failed | JobStatus::Cancelled
        )
        || job.progress().percent == 100
    {
        return Err(invalid(
            "Assembly update requires a new non-completed job revision",
        ));
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(database)?;
    persist_job(&mut tx, job, attempt, expected).await?;
    if matches!(job.status(), JobStatus::Failed | JobStatus::Cancelled) {
        sqlx::query("UPDATE audio_assembly_attempts SET ended_at=? WHERE attempt_id=? AND job_id=? AND ended_at IS NULL")
            .bind(chrono::Utc::now().to_rfc3339()).bind(attempt.to_string()).bind(job.id().to_string()).execute(&mut *tx).await.map_err(database)?;
    }
    tx.commit().await.map_err(database)
}

pub(super) async fn persist_job(
    tx: &mut Transaction<'_, Sqlite>,
    job: &Job,
    attempt: Uuid,
    expected: u64,
) -> Result<(), PortError> {
    let row = job_to_row_values(job)?;
    let changed = sqlx::query("UPDATE jobs SET revision=?,status=?,stage=?,progress_json=?,error_json=?,updated_at=?,started_at=?,finished_at=? WHERE id=? AND project_id=? AND kind='audioAssembly' AND revision=? AND title=? AND created_at=? AND (started_at IS NULL OR started_at IS ?) AND status IN('pending','running','cancelling') AND (status!='cancelling' OR ? IN('failed','cancelled')) AND EXISTS(SELECT 1 FROM audio_assembly_attempts a WHERE a.job_id=jobs.id AND a.attempt_id=? AND a.ended_at IS NULL)")
        .bind(row.revision).bind(&row.status).bind(row.stage).bind(row.progress_json).bind(row.error_json).bind(row.updated_at).bind(&row.started_at).bind(row.finished_at)
        .bind(row.id).bind(row.project_id).bind(integer(expected)?).bind(row.title).bind(row.created_at).bind(&row.started_at).bind(&row.status).bind(attempt.to_string())
        .execute(&mut **tx).await.map_err(database)?;
    if changed.rows_affected() != 1 {
        return Err(conflict(
            "Assembly job revision, cancellation or attempt changed",
        ));
    }
    Ok(())
}

pub(crate) async fn close_assembly_attempts(pool: &SqlitePool) -> Result<(), PortError> {
    sqlx::query("UPDATE audio_assembly_attempts SET ended_at=? WHERE ended_at IS NULL AND EXISTS(SELECT 1 FROM jobs j WHERE j.id=audio_assembly_attempts.job_id AND j.status IN('failed','cancelled','completed'))")
        .bind(chrono::Utc::now().to_rfc3339()).execute(pool).await.map_err(database)?;
    Ok(())
}
