use super::{
    AudioAssemblyUseCases,
    computed::ComputedAssembly,
    error::{conflict, invalid},
    request::InspectAssemblyRequest,
};
use crate::error::ApplicationError;
use ports::{
    audio_inspection::SpeechTimingSettings,
    audio_jobs::{AudioAssemblyIntent, AudioAssemblyPlan, MAX_ASSEMBLY_PLAN_BYTES},
    cancellation::CancellationToken,
};
use sha2::{Digest, Sha256};

impl AudioAssemblyUseCases {
    pub(super) fn stored_plan(
        &self,
        intent: &AudioAssemblyIntent,
    ) -> Result<AudioAssemblyPlan, ApplicationError> {
        if intent.plan_json.len() > MAX_ASSEMBLY_PLAN_BYTES
            || format!("{:x}", Sha256::digest(intent.plan_json.as_bytes()))
                != intent.plan_sha256.as_str()
        {
            return Err(invalid("Stored audio assembly plan bytes changed").into());
        }
        let plan: AudioAssemblyPlan = serde_json::from_str(&intent.plan_json)
            .map_err(|_| invalid("Stored audio assembly plan JSON is invalid"))?;
        if plan.project_id != intent.project_id
            || plan.source_run_id != intent.source_run_id
            || plan.source_attempt_id != intent.source_attempt_id
            || plan.assembly_policy != "pcm16_48k_exact_concat_v1"
            || plan.ffmpeg_sha256 != self.ffmpeg_sha256
        {
            return Err(conflict(
                "Assembly owner, source, policy or managed FFmpeg changed; review a new preview",
            )
            .into());
        }
        Ok(plan)
    }

    pub(super) async fn recheck(
        &self,
        intent: &AudioAssemblyIntent,
        cancel: CancellationToken,
    ) -> Result<ComputedAssembly, ApplicationError> {
        let plan = self.stored_plan(intent)?;
        let computed = self
            .compute(
                intent.project_id.clone(),
                InspectAssemblyRequest {
                    video_artifact_id: plan
                        .video
                        .as_ref()
                        .map(|video| video.artifact.artifact_id.clone()),
                    source_run_id: intent.source_run_id,
                    settings: SpeechTimingSettings {
                        mode: plan.timeline.mode,
                        source_duration_ms: plan.timeline.source_duration_ms,
                        pause_after_ms: plan
                            .timeline
                            .utterances
                            .iter()
                            .map(|utterance| utterance.pause_after_ms)
                            .collect(),
                    },
                },
                cancel,
            )
            .await?;
        if computed.intent.plan_json != intent.plan_json
            || computed.intent.plan_sha256 != intent.plan_sha256
            || !computed.inspection.conflicts.is_empty()
        {
            return Err(conflict(
                "Managed source or timing facts changed since review; inspect a new preview",
            )
            .into());
        }
        Ok(computed)
    }
}
