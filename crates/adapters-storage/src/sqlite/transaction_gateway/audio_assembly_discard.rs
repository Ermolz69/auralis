use super::audio_assembly_publication::{clean_key, load_intent};
use crate::sqlite::{
    audio_assembly_store::{
        error::{database, integer, invalid},
        validation,
    },
    unit_of_work::outbox_writes::save_outbox_message,
};
use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::{PortError, audio_jobs::AudioAssemblyPublication};
use sqlx::SqlitePool;
use uuid::Uuid;

pub(crate) async fn discard_assembly_publication(
    pool: &SqlitePool,
    id: Uuid,
    attempt: Uuid,
) -> Result<bool, PortError> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(database)?;
    let json:Option<String> = sqlx::query_scalar("SELECT publication_json FROM audio_assembly_publications WHERE assembly_id=? AND attempt_id=? AND ready=0")
        .bind(id.to_string()).bind(attempt.to_string()).fetch_optional(&mut *tx).await.map_err(database)?;
    let Some(json) = json else {
        return Ok(false);
    };
    let intent = load_intent(&mut tx, id).await?;
    let publication: AudioAssemblyPublication = serde_json::from_str(&json)
        .map_err(|_| invalid("Discarded assembly publication JSON is invalid"))?;
    validation::publication(&publication, &validation::intent(&intent)?)?;
    if publication.assembly_id != id || publication.attempt_id != attempt {
        return Err(invalid("Discarded publication identity differs"));
    }
    for file in publication.files {
        let associated:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audio_assembly_artifacts WHERE artifact_id=? AND assembly_id=? AND attempt_id=? AND role=? AND sha256=? AND size_bytes=?)")
            .bind(file.artifact_id.to_string()).bind(id.to_string()).bind(attempt.to_string()).bind(&file.kind).bind(&file.sha256).bind(integer(file.size_bytes)?)
            .fetch_one(&mut *tx).await.map_err(database)?;
        if !associated {
            return Err(invalid(
                "Discarded publication lost its normalized output identity",
            ));
        }
        let payload:String = sqlx::query_scalar("SELECT payload_json FROM outbox_messages WHERE json_extract(payload_json,'$.type')='finalize_staged_artifact' AND json_extract(payload_json,'$.artifact_id')=? LIMIT 1")
            .bind(file.artifact_id.to_string()).fetch_optional(&mut *tx).await.map_err(database)?.ok_or_else(|| invalid("Discarded assembly has no finalization intent"))?;
        let payload: OutboxPayload = serde_json::from_str(&payload)
            .map_err(|_| invalid("Discarded assembly finalization intent is invalid"))?;
        let OutboxPayload::FinalizeStagedArtifact {
            project_id,
            artifact_id,
            staging_key,
            final_key,
        } = payload
        else {
            return Err(invalid(
                "Discarded assembly has a different finalization intent",
            ));
        };
        if project_id != intent.project_id
            || artifact_id != file.artifact_id
            || !clean_key(&staging_key)
            || !clean_key(&final_key)
            || !staging_key.starts_with(".staging/")
            || !final_key.starts_with(&format!("{}/", intent.project_id))
        {
            return Err(invalid(
                "Discarded assembly finalization owner or keys differ",
            ));
        }
        sqlx::query("INSERT INTO audio_assembly_discarded_artifacts VALUES (?,?,?,?,?)")
            .bind(file.artifact_id.to_string())
            .bind(id.to_string())
            .bind(attempt.to_string())
            .bind(&final_key)
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(database)?;
        for storage_key in [staging_key, final_key] {
            save_outbox_message(
                &mut tx,
                &OutboxMessage::new(OutboxPayload::DeleteStorageKey { storage_key }),
            )
            .await?;
        }
        sqlx::query("DELETE FROM artifacts WHERE id=? AND project_id=?")
            .bind(file.artifact_id.to_string())
            .bind(intent.project_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(database)?;
    }
    sqlx::query(
        "DELETE FROM audio_assembly_publications WHERE assembly_id=? AND attempt_id=? AND ready=0",
    )
    .bind(id.to_string())
    .bind(attempt.to_string())
    .execute(&mut *tx)
    .await
    .map_err(database)?;
    tx.commit().await.map_err(database)?;
    Ok(true)
}
