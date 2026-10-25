use super::error::{integer, invalid};
use domain::speech::timeline::plan_audio_timeline;
use ports::{
    PortError,
    audio_jobs::{
        AudioAssemblyIntent, AudioAssemblyPlan, AudioAssemblyPublication, MAX_ASSEMBLY_PLAN_BYTES,
    },
};
use sha2::{Digest, Sha256};

pub(crate) fn intent(intent: &AudioAssemblyIntent) -> Result<AudioAssemblyPlan, PortError> {
    if intent.plan_json.is_empty()
        || intent.plan_json.len() > MAX_ASSEMBLY_PLAN_BYTES
        || format!("{:x}", Sha256::digest(intent.plan_json.as_bytes()))
            != intent.plan_sha256.as_str()
    {
        return Err(invalid(
            "Immutable assembly plan bytes or digest are invalid",
        ));
    }
    let plan: AudioAssemblyPlan = serde_json::from_str(&intent.plan_json)
        .map_err(|_| invalid("Immutable assembly plan JSON is invalid"))?;
    if plan.project_id != intent.project_id
        || plan.source_run_id != intent.source_run_id
        || plan.source_attempt_id != intent.source_attempt_id
        || plan.raw_manifest.sha256 != plan.timeline.raw_manifest_sha256
        || plan.assembly_policy != "pcm16_48k_exact_concat_v1"
        || plan.raw_wav.artifact_id == plan.raw_manifest.artifact_id
        || plan.raw_wav.artifact_id == plan.layout.artifact_id
        || plan.raw_manifest.artifact_id == plan.layout.artifact_id
    {
        return Err(invalid(
            "Immutable assembly plan ownership, source or policy differs",
        ));
    }
    for file in [&plan.raw_wav, &plan.raw_manifest, &plan.layout] {
        if file.size_bytes == 0 {
            return Err(invalid("Assembly source artifact is empty"));
        }
        integer(file.size_bytes)?;
    }
    if plan.raw_wav.size_bytes > 1_u64 << 32
        || plan.raw_manifest.size_bytes > MAX_ASSEMBLY_PLAN_BYTES as u64
        || plan.layout.size_bytes > MAX_ASSEMBLY_PLAN_BYTES as u64
        || plan
            .timeline
            .utterances
            .iter()
            .any(|utterance| utterance.sample_rate != 48000)
        || plan_audio_timeline(&plan.timeline).is_err()
    {
        return Err(invalid(
            "Assembly source limits, measurements or timing are invalid",
        ));
    }
    if let Some(video) = &plan.video {
        let frames = plan_audio_timeline(&plan.timeline)
            .map_err(|_| invalid("Invalid video timing"))?
            .frame_count;
        if video.policy_id != ports::video_export::REPLACE_AUDIO_POLICY
            || video.artifact.sha256 != video.media.sha256
            || video.artifact.size_bytes != video.media.size_bytes
            || video.artifact.size_bytes == 0
            || video.artifact.size_bytes > 1_u64 << 32
            || video.media.duration_ms == 0
            || video.media.duration_ms > 28800000
            || frames > video.media.duration_ms * 48
        {
            return Err(invalid(
                "Reviewed video source or speech coverage is invalid",
            ));
        }
    }
    Ok(plan)
}

pub(crate) fn publication(
    value: &AudioAssemblyPublication,
    plan: &AudioAssemblyPlan,
) -> Result<(), PortError> {
    let timeline =
        plan_audio_timeline(&plan.timeline).map_err(|_| invalid("Stored timing now conflicts"))?;
    if value.ready
        || value.sample_rate != 48000
        || value.channels != timeline.channels
        || value.frames != timeline.frame_count
        || value.peak_sample_abs > 32768
        || value.full_scale_samples > value.frames * u64::from(value.channels)
        || value.files.len() != if plan.video.is_some() { 4 } else { 2 }
        || value.files.iter().filter(|file| file.kind == "wav").count() != 1
        || value
            .files
            .iter()
            .filter(|file| file.kind == "manifest")
            .count()
            != 1
        || value.files[0].artifact_id == value.files[1].artifact_id
        || value.files.iter().any(|file| {
            file.size_bytes == 0
                || domain::speech::Sha256Digest::parse(file.sha256.clone()).is_err()
        })
    {
        return Err(invalid(
            "Assembly publication does not cover its immutable timing plan",
        ));
    }
    let roles = if plan.video.is_some() {
        vec!["wav", "manifest", "video", "video_manifest"]
    } else {
        vec!["wav", "manifest"]
    };
    if roles
        .iter()
        .any(|role| value.files.iter().filter(|file| &file.kind == role).count() != 1)
        || value.files.iter().enumerate().any(|(index, file)| {
            value.files[..index]
                .iter()
                .any(|other| other.artifact_id == file.artifact_id)
        })
    {
        return Err(invalid(
            "Assembly publication has missing, extra or repeated outputs",
        ));
    }
    for file in &value.files {
        integer(file.size_bytes)?;
    }
    if value.files.iter().any(|file| {
        [&plan.raw_wav, &plan.raw_manifest, &plan.layout]
            .iter()
            .any(|source| source.artifact_id == file.artifact_id)
    }) {
        return Err(invalid(
            "Assembly output cannot replace a raw source artifact",
        ));
    }
    Ok(())
}
