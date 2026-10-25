use super::audio_assembly_attempt::persist_job;
use crate::sqlite::{
    audio_assembly_store::{
        error::{conflict, database, integer, invalid},
        mapper,
        row::AssemblyIntentRow,
        validation,
    },
    job_mapper::row_to_job,
    job_row::JobRow,
    unit_of_work::{insert_new_artifact, outbox_writes::save_outbox_message},
};
use domain::{
    job::{Job, JobKind, JobStatus},
    media::{ArtifactKind, ArtifactLocation, ArtifactState},
    outbox::{OutboxMessage, OutboxPayload},
};
use ports::{PortError, audio_jobs::AudioAssemblyPublication, storage::StagedArtifact};
use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

pub(crate) async fn publish_assembly(
    pool: &SqlitePool,
    job: &Job,
    publication: &AudioAssemblyPublication,
    staged: &[StagedArtifact],
) -> Result<(), PortError> {
    if job.kind() != &JobKind::AudioAssembly
        || job.status() != &JobStatus::Running
        || !matches!(staged.len(), 2 | 4)
    {
        return Err(invalid(
            "Assembly publication requires its running job and a complete staged file set",
        ));
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(database)?;
    let intent = load_intent(&mut tx, publication.assembly_id).await?;
    let plan = validation::intent(&intent)?;
    validation::publication(publication, &plan)?;
    let eligible:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audio_assembly_attempts a JOIN jobs j ON j.id=a.job_id WHERE a.assembly_id=? AND a.attempt_id=? AND a.job_id=? AND a.ended_at IS NULL AND j.project_id=? AND j.kind='audioAssembly' AND j.status='running' AND j.revision=?)")
        .bind(publication.assembly_id.to_string()).bind(publication.attempt_id.to_string()).bind(job.id().to_string()).bind(intent.project_id.to_string()).bind(integer(job.revision())?)
        .fetch_one(&mut *tx).await.map_err(database)?;
    if !eligible || job.project_id() != &intent.project_id {
        return Err(conflict(
            "Assembly publication lost its active attempt or owner",
        ));
    }
    for (artifact, file) in staged.iter().zip(&publication.files) {
        let kind = match file.kind.as_str() {
            "wav" => ArtifactKind::MixedAudio,
            "video" => ArtifactKind::FinalVideo,
            "manifest" | "video_manifest" => ArtifactKind::SpeechManifest,
            _ => return Err(invalid("Unknown assembly output role")),
        };
        if artifact.artifact.id != file.artifact_id
            || artifact.size_bytes != file.size_bytes
            || artifact.artifact.size_bytes != Some(file.size_bytes)
            || artifact.artifact.state != ArtifactState::PendingFinalize
            || artifact.artifact.kind != kind
            || artifact.artifact.location
                != ArtifactLocation::StorageKey(artifact.final_key.clone())
            || !clean_key(&artifact.final_key)
            || !artifact
                .final_key
                .starts_with(&format!("{}/", intent.project_id))
            || !clean_key(&artifact.staging_key)
            || !artifact.staging_key.starts_with(".staging/")
        {
            return Err(invalid(
                "Assembly staged artifact differs from its managed publication",
            ));
        }
        insert_new_artifact(&mut tx, &intent.project_id, &artifact.artifact).await?;
        sqlx::query("INSERT INTO audio_assembly_artifacts VALUES (?,?,?,?,?,?)")
            .bind(file.artifact_id.to_string())
            .bind(publication.assembly_id.to_string())
            .bind(publication.attempt_id.to_string())
            .bind(&file.kind)
            .bind(&file.sha256)
            .bind(integer(file.size_bytes)?)
            .execute(&mut *tx)
            .await
            .map_err(database)?;
        save_outbox_message(
            &mut tx,
            &OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
                project_id: intent.project_id.clone(),
                artifact_id: file.artifact_id.clone(),
                staging_key: artifact.staging_key.clone(),
                final_key: artifact.final_key.clone(),
            }),
        )
        .await?;
    }
    let json = serde_json::to_string(publication)
        .map_err(|_| invalid("Assembly publication cannot be serialized"))?;
    if json.len() > 65536 {
        return Err(invalid(
            "Assembly publication metadata exceeds its byte ceiling",
        ));
    }
    sqlx::query("INSERT INTO audio_assembly_publications VALUES (?,?,?,0,?)")
        .bind(publication.assembly_id.to_string())
        .bind(publication.attempt_id.to_string())
        .bind(json)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)
}

pub(crate) async fn complete_assembly_publication(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<Job>, PortError> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(database)?;
    let json: Option<String> = sqlx::query_scalar(
        "SELECT publication_json FROM audio_assembly_publications WHERE assembly_id=? AND ready=0",
    )
    .bind(id.to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(database)?;
    let Some(json) = json else {
        return Ok(None);
    };
    let publication: AudioAssemblyPublication = serde_json::from_str(&json)
        .map_err(|_| invalid("Stored assembly publication is invalid"))?;
    let intent = load_intent(&mut tx, id).await?;
    validation::publication(&publication, &validation::intent(&intent)?)?;
    if publication.assembly_id != id {
        return Err(invalid("Stored assembly publication identity changed"));
    }
    let row = sqlx::query_as::<_, JobRow>("SELECT j.* FROM jobs j JOIN audio_assembly_attempts a ON a.job_id=j.id WHERE a.assembly_id=? AND a.attempt_id=? AND a.ended_at IS NULL AND j.kind='audioAssembly' AND j.status='running'")
        .bind(id.to_string()).bind(publication.attempt_id.to_string()).fetch_optional(&mut *tx).await.map_err(database)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut job = row_to_job(row)?;
    if job.project_id() != &intent.project_id {
        return Err(invalid("Assembly completion project changed"));
    }
    for file in &publication.files {
        let ready:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audio_assembly_artifacts l JOIN artifacts a ON a.id=l.artifact_id WHERE l.assembly_id=? AND l.attempt_id=? AND l.artifact_id=? AND l.role=? AND l.sha256=? AND l.size_bytes=? AND a.state='ready' AND a.project_id=? AND a.size_bytes=l.size_bytes)")
            .bind(id.to_string()).bind(publication.attempt_id.to_string()).bind(file.artifact_id.to_string()).bind(&file.kind).bind(&file.sha256).bind(integer(file.size_bytes)?).bind(intent.project_id.to_string())
            .fetch_one(&mut *tx).await.map_err(database)?;
        if !ready {
            return Ok(None);
        }
    }
    let revision = job.revision();
    job.mark_completed()
        .map_err(|error| invalid(error.to_string()))?;
    persist_job(&mut tx, &job, publication.attempt_id, revision).await?;
    sqlx::query(
        "UPDATE audio_assembly_attempts SET ended_at=? WHERE attempt_id=? AND ended_at IS NULL",
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(publication.attempt_id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(database)?;
    sqlx::query("UPDATE audio_assembly_publications SET ready=1 WHERE assembly_id=? AND attempt_id=? AND ready=0")
        .bind(id.to_string()).bind(publication.attempt_id.to_string()).execute(&mut *tx).await.map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(Some(job))
}

pub(super) async fn load_intent(
    tx: &mut Transaction<'_, Sqlite>,
    id: Uuid,
) -> Result<ports::audio_jobs::AudioAssemblyIntent, PortError> {
    let row = sqlx::query_as::<_, AssemblyIntentRow>("SELECT assembly_id,project_id,source_run_id,source_attempt_id,plan_json,plan_sha256 FROM audio_assembly_intents WHERE assembly_id=?")
        .bind(id.to_string()).fetch_optional(&mut **tx).await.map_err(database)?.ok_or_else(|| invalid("Assembly intent is missing"))?;
    mapper::intent(row)
}

pub(super) fn clean_key(key: &str) -> bool {
    !key.contains(['\\', ':']) && key.split('/').all(|part| !matches!(part, "" | "." | ".."))
}
