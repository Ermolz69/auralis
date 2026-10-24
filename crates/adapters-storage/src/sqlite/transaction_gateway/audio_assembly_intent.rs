use super::audio_assembly_source::require_ready_source;
use crate::sqlite::audio_assembly_store::{
    error::{conflict, database},
    mapper,
    row::AssemblyIntentRow,
    validation,
};
use ports::{PortError, audio_jobs::AudioAssemblyIntent};
use sqlx::SqlitePool;

pub(crate) async fn register_assembly(
    pool: &SqlitePool,
    intent: &AudioAssemblyIntent,
) -> Result<AudioAssemblyIntent, PortError> {
    let plan = validation::intent(intent)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await.map_err(database)?;
    require_ready_source(&mut tx, &plan).await?;
    sqlx::query("INSERT OR IGNORE INTO audio_assembly_intents VALUES (?,?,?,?,?,?,?)")
        .bind(intent.assembly_id.to_string())
        .bind(intent.project_id.to_string())
        .bind(intent.source_run_id.to_string())
        .bind(intent.source_attempt_id.to_string())
        .bind(&intent.plan_json)
        .bind(intent.plan_sha256.as_str())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&mut *tx)
        .await
        .map_err(database)?;
    let row = sqlx::query_as::<_, AssemblyIntentRow>("SELECT assembly_id,project_id,source_run_id,source_attempt_id,plan_json,plan_sha256 FROM audio_assembly_intents WHERE project_id=? AND plan_sha256=?")
        .bind(intent.project_id.to_string()).bind(intent.plan_sha256.as_str()).fetch_optional(&mut *tx).await.map_err(database)?.ok_or_else(|| conflict("Assembly ID already names a different immutable plan"))?;
    let retained = mapper::intent(row)?;
    if retained.plan_json != intent.plan_json {
        return Err(conflict("Assembly plan bytes differ for the same digest"));
    }
    tx.commit().await.map_err(database)?;
    Ok(retained)
}
