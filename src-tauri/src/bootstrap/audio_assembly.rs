use super::{assembly_media, paths::AppPaths, services::RuntimeServices};
use adapters_ffmpeg::assembly::{
    AudioAssemblyLimits, FfmpegAudioAssembler, ManagedSpeechAudioReader,
};
use application::usecases::audio_assembly::AudioAssemblyUseCases;
use domain::speech::Sha256Digest;
use ports::{
    job_runtime_control::JobRuntimeControlPort, job_scheduler::JobLifecycleEvent,
    speech_audio::SpeechAudioAssemblyPort,
};
use std::sync::Arc;
use tauri::AppHandle;

pub fn setup(
    app: &AppHandle,
    paths: &AppPaths,
    services: &RuntimeServices,
    runtime: Arc<dyn JobRuntimeControlPort>,
    emitter: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
) -> Result<Arc<AudioAssemblyUseCases>, Box<dyn std::error::Error>> {
    let limits = AudioAssemblyLimits::default();
    let inspector = Arc::new(ManagedSpeechAudioReader::new(
        paths.projects(),
        limits.clone(),
    )?);
    let staged = paths.workspaces().join("audio-assembly");
    std::fs::create_dir_all(&staged)?;
    let configured = assembly_media::pinned_ffmpeg(app).and_then(|(path, pin)| {
        FfmpegAudioAssembler::new(path, pin, paths.projects(), staged, limits)
            .map_err(|error| error.to_string())
    });
    let (renderer, renderer_error): (Option<Arc<dyn SpeechAudioAssemblyPort>>, Option<String>) =
        match configured {
            Ok(renderer) => (Some(Arc::new(renderer)), None),
            Err(error) => (None, Some(error)),
        };
    Ok(Arc::new(AudioAssemblyUseCases {
        projects: services.project_repo.clone(),
        speech: services.speech.clone(),
        store: services.audio_assembly.clone(),
        index: services.artifact_index.clone(),
        artifacts: services.artifact_store.clone(),
        inspector,
        renderer,
        renderer_error,
        ffmpeg_sha256: Sha256Digest::parse(assembly_media::WINDOWS_FFMPEG_SHA256)?,
        runtime,
        emitter,
    }))
}
