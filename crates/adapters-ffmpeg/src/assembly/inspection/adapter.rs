use super::super::{
    AudioAssemblyLimits,
    control::{CancelOnDrop, WorkControl},
    error, source,
};
use super::{files, manifest, measurement};
use async_trait::async_trait;
use domain::speech::timeline::plan_audio_timeline;
use ports::{
    PortError,
    audio_inspection::{
        SpeechAudioInspection, SpeechAudioInspectionPort, SpeechAudioInspectionRequest,
    },
    cancellation::CancellationToken,
    speech_audio::SpeechAudioLayout,
};
use std::{fs, path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct ManagedSpeechAudioReader {
    root: PathBuf,
    limits: AudioAssemblyLimits,
    admission: std::sync::Arc<tokio::sync::Semaphore>,
}

impl ManagedSpeechAudioReader {
    #[cfg(test)]
    pub(super) fn admission_for_test(&self) -> std::sync::Arc<tokio::sync::Semaphore> {
        self.admission.clone()
    }
    pub fn new(root: PathBuf, limits: AudioAssemblyLimits) -> Result<Self, PortError> {
        limits.validate()?;
        if !root.is_absolute() {
            return Err(error::invalid("Managed speech input root must be absolute"));
        }
        Ok(Self {
            root: fs::canonicalize(root).map_err(error::io)?,
            limits,
            admission: std::sync::Arc::new(tokio::sync::Semaphore::new(1)),
        })
    }

    fn inspect_owned(
        &self,
        request: SpeechAudioInspectionRequest,
        control: WorkControl,
    ) -> Result<SpeechAudioInspection, PortError> {
        control.check()?;
        let mut layout: SpeechAudioLayout = files::json(&request.layout, &self.root, &control)?;
        source::validate(&layout)?;
        if layout.source_plan_sha256 != request.source_plan_sha256
            || layout.raw_manifest_sha256 != request.manifest.sha256
            || layout.aggregate_sha256 != request.aggregate.sha256
            || layout
                .utterances
                .iter()
                .map(|range| &range.utterance_id)
                .ne(request.expected_utterance_ids.iter())
        {
            return Err(error::invalid(
                "Managed layout differs from the selected immutable speech result",
            ));
        }
        let raw = files::json(&request.manifest, &self.root, &control)?;
        manifest::verify(&raw, &layout, &control)?;
        let mut aggregate =
            files::open_owned(&request.aggregate, &self.root, self.limits.max_input_bytes)?;
        files::verify(&mut aggregate, &request.aggregate, false, &control)?;
        let properties = common::wav::inspect_pcm16(&mut aggregate).map_err(error::wave)?;
        if properties.sample_rate != layout.sample_rate
            || properties.channels != layout.channels
            || properties.frames != layout.frame_count
        {
            return Err(error::invalid(
                "Ready managed WAV measurements differ from the verified layout",
            ));
        }
        let timeline_request = measurement::measure(
            &mut aggregate,
            &properties,
            &layout,
            request.settings,
            &control,
        )?;
        files::verify(&mut aggregate, &request.aggregate, false, &control)?;
        layout.aggregate_path = fs::canonicalize(request.aggregate.path).map_err(error::io)?;
        let (timeline, conflicts) = match plan_audio_timeline(&timeline_request) {
            Ok(timeline) => (Some(timeline), Vec::new()),
            Err(conflicts) => (None, conflicts),
        };
        control.check()?;
        Ok(SpeechAudioInspection {
            source: layout,
            timeline_request,
            timeline,
            conflicts,
        })
    }
}

#[async_trait]
impl SpeechAudioInspectionPort for ManagedSpeechAudioReader {
    async fn inspect(
        &self,
        request: SpeechAudioInspectionRequest,
        cancel: CancellationToken,
    ) -> Result<SpeechAudioInspection, PortError> {
        if cancel.is_cancelled() {
            return Err(PortError::Cancelled);
        }
        let cancel = cancel.child_token();
        let guard = CancelOnDrop(cancel.clone());
        let limit = self.limits.total_timeout.min(Duration::from_secs(120));
        let control = WorkControl::new(cancel.clone(), limit)?;
        let permit = tokio::select! {
            _ = cancel.cancelled() => return Err(PortError::Cancelled),
            result = tokio::time::timeout(limit, self.admission.clone().acquire_owned()) => {
                result.map_err(|_| PortError::Busy { message:"Speech inspection queue exceeded its 120-second deadline".into() })?
                    .map_err(|_| error::invalid("Speech inspection admission was closed"))?
            }
        };
        let reader = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            reader.inspect_owned(request, control)
        })
        .await
        .map_err(|error| super::super::error::external(error.to_string()))?;
        drop(guard);
        result
    }
}
