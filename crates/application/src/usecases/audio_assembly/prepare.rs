use super::{
    AudioAssemblyUseCases, AudioAssemblyView, InspectAssemblyRequest, PrepareAssemblyRequest,
    error::{conflict, invalid},
};
use crate::error::ApplicationError;
use domain::{
    project::ProjectId,
    speech::{
        Sha256Digest,
        timeline::{MeasuredUtterance, TimelineConflict},
    },
};
use ports::{audio_inspection::SpeechTimingSettings, cancellation::CancellationToken};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct AudioAssemblyPreview {
    pub source_run_id: Uuid,
    pub source_plan_sha256: Sha256Digest,
    pub preview_sha256: Sha256Digest,
    pub profile_id: String,
    pub settings: SpeechTimingSettings,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: Option<u64>,
    pub utterances: Vec<MeasuredUtterance>,
    pub conflicts: Vec<TimelineConflict>,
    pub renderer_available: bool,
    pub renderer_error: Option<String>,
}

impl AudioAssemblyUseCases {
    pub async fn inspect(
        &self,
        project: ProjectId,
        request: InspectAssemblyRequest,
        cancel: CancellationToken,
    ) -> Result<AudioAssemblyPreview, ApplicationError> {
        let settings = request.settings.clone();
        let computed = self.compute(project, request, cancel).await?;
        Ok(AudioAssemblyPreview {
            source_run_id: computed.intent.source_run_id,
            source_plan_sha256: computed.plan.timeline.source_plan_sha256,
            preview_sha256: computed.intent.plan_sha256,
            profile_id: computed.profile_id,
            settings,
            sample_rate: 48000,
            channels: computed.inspection.source.channels,
            frames: computed
                .inspection
                .timeline
                .as_ref()
                .map(|timeline| timeline.frame_count),
            utterances: computed.inspection.timeline_request.utterances,
            conflicts: computed.inspection.conflicts,
            renderer_available: self.renderer.is_some(),
            renderer_error: self.renderer_error.clone(),
        })
    }

    pub async fn prepare(
        &self,
        project: ProjectId,
        request: PrepareAssemblyRequest,
    ) -> Result<AudioAssemblyView, ApplicationError> {
        if !request.reviewed {
            return Err(
                invalid("Confirm the exact source and timing preview before assembly").into(),
            );
        }
        let computed = self
            .compute(project, request.source, CancellationToken::new())
            .await?;
        if computed.intent.plan_sha256 != request.expected_preview_sha256 {
            return Err(conflict(
                "Source or timing settings changed; inspect and review the new preview",
            )
            .into());
        }
        if !computed.inspection.conflicts.is_empty() {
            return Err(
                invalid("Resolve the displayed timing conflicts before audio assembly").into(),
            );
        }
        let intent = self.store.register(&computed.intent).await?;
        self.view(intent).await
    }
}
