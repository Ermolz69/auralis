use domain::speech::Sha256Digest;
use ports::{
    artifact_index::ArtifactIndex, audio_inspection::SpeechAudioInspectionPort,
    audio_jobs::AudioAssemblyStore, job_runtime_control::JobRuntimeControlPort,
    job_scheduler::JobLifecycleEvent, repository::ProjectRepository,
    speech_audio::SpeechAudioAssemblyPort, speech_workflow::SpeechWorkflowStore,
    storage::ArtifactStore,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AudioAssemblyUseCases {
    pub projects: Arc<dyn ProjectRepository>,
    pub speech: Arc<dyn SpeechWorkflowStore>,
    pub store: Arc<dyn AudioAssemblyStore>,
    pub index: Arc<dyn ArtifactIndex>,
    pub artifacts: Arc<dyn ArtifactStore>,
    pub inspector: Arc<dyn SpeechAudioInspectionPort>,
    pub renderer: Option<Arc<dyn SpeechAudioAssemblyPort>>,
    pub renderer_error: Option<String>,
    pub video_export: Option<Arc<dyn ports::video_export::VideoExportPort>>,
    pub ffmpeg_sha256: Sha256Digest,
    pub runtime: Arc<dyn JobRuntimeControlPort>,
    pub emitter: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
}
