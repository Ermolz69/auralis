use crate::sqlite::{
    audio_assembly_store::error::{conflict, database, integer},
    helpers::serialize_enum,
};
use domain::media::ArtifactKind;
use ports::{PortError, audio_jobs::AudioAssemblyPlan};
use sqlx::{Sqlite, Transaction};

pub(super) async fn require_ready_source(
    tx: &mut Transaction<'_, Sqlite>,
    plan: &AudioAssemblyPlan,
) -> Result<(), PortError> {
    let ids = serde_json::to_string(&plan.timeline.expected_utterance_ids)
        .map_err(|_| conflict("Invalid source utterance identities"))?;
    let ready:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM speech_run_intents i JOIN speech_publications p ON p.run_id=i.run_id WHERE i.run_id=? AND i.project_id=? AND i.plan_sha256=? AND p.attempt_id=? AND p.ready=1 AND i.expected_utterances=? AND json_array_length(i.plan_json,'$.utterances')=i.expected_utterances AND NOT EXISTS(SELECT 1 FROM json_each(i.plan_json,'$.utterances') u WHERE json_extract(u.value,'$.utterance_id') IS NOT json_extract(?,'$['||u.key||']')))")
        .bind(plan.source_run_id.to_string()).bind(plan.project_id.to_string()).bind(plan.timeline.source_plan_sha256.as_str()).bind(plan.source_attempt_id.to_string())
        .bind(plan.timeline.expected_utterance_ids.len() as i64).bind(ids).fetch_one(&mut **tx).await.map_err(database)?;
    if !ready {
        return Err(conflict(
            "Selected raw speech result is not ready or its immutable plan changed",
        ));
    }
    for (kind, file, artifact_kind) in [
        ("wav", &plan.raw_wav, ArtifactKind::GeneratedSpeech),
        ("manifest", &plan.raw_manifest, ArtifactKind::SpeechManifest),
        ("audio_layout", &plan.layout, ArtifactKind::SpeechManifest),
    ] {
        let present:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM speech_publications p, json_each(p.publication_json,'$.files') f JOIN artifacts a ON a.id=json_extract(f.value,'$.artifact_id') WHERE p.run_id=? AND p.attempt_id=? AND p.ready=1 AND json_extract(f.value,'$.kind')=? AND a.id=? AND json_extract(f.value,'$.sha256')=? AND json_extract(f.value,'$.size_bytes')=? AND a.project_id=? AND a.kind=? AND a.state='ready' AND a.size_bytes=?)")
            .bind(plan.source_run_id.to_string()).bind(plan.source_attempt_id.to_string()).bind(kind).bind(file.artifact_id.to_string()).bind(file.sha256.as_str())
            .bind(integer(file.size_bytes)?).bind(plan.project_id.to_string()).bind(serialize_enum(&artifact_kind, "assembly source artifact")?).bind(integer(file.size_bytes)?)
            .fetch_one(&mut **tx).await.map_err(database)?;
        if !present {
            return Err(conflict(
                "Selected raw speech artifact identity, ownership or readiness changed",
            ));
        }
    }
    Ok(())
}
