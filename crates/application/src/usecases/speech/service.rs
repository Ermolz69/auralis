use super::super::translation::{InspectTranslationSourceUseCase, TranslationReviewUseCase};
use ports::{
    artifact_index::ArtifactIndex,
    job_runtime_control::JobRuntimeControlPort,
    job_scheduler::JobLifecycleEvent,
    repository::ProjectRepository,
    speech_catalog::SpeechCatalogPort,
    speech_workflow::{SpeechExecutionPort, SpeechWorkflowStore},
    storage::ArtifactStore,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct SpeechUseCases {
    pub projects: Arc<dyn ProjectRepository>,
    pub store: Arc<dyn SpeechWorkflowStore>,
    pub engine: Option<Arc<dyn SpeechExecutionPort>>,
    pub catalog: Option<Arc<dyn SpeechCatalogPort>>,
    pub runtime_error: Option<String>,
    pub runtime: Arc<dyn JobRuntimeControlPort>,
    pub artifacts: Arc<dyn ArtifactStore>,
    pub index: Arc<dyn ArtifactIndex>,
    pub translation_review: Arc<TranslationReviewUseCase>,
    pub subtitles: Arc<InspectTranslationSourceUseCase>,
    pub emitter: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
}
