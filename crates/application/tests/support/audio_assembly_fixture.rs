use super::source::{self, AssemblySource};
use adapters_ffmpeg::assembly::{AudioAssemblyLimits, ManagedSpeechAudioReader};
use adapters_storage::{
    local::LocalTempWorkspace,
    sqlite::{
        SqliteAudioAssemblyStore, SqliteJobRepository, SqliteOutboxRepository,
        SqliteProjectRepository, SqliteStorageUnitOfWork,
    },
};
use application::{
    usecases::audio_assembly::{
        AudioAssemblyUseCases, AudioAssemblyView, InspectAssemblyRequest, PrepareAssemblyRequest,
    },
    worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig},
};
use async_trait::async_trait;
use domain::{
    job::JobStatus,
    speech::{Sha256Digest, TimingMode},
};
use ports::{
    PortError,
    audio_assembly::StagedAudioAssembly,
    audio_inspection::SpeechTimingSettings,
    cancellation::CancellationToken,
    job_runtime_control::JobRuntimeControlPort,
    speech_audio::{SpeechAudioAssemblyPort, SpeechAudioAssemblyRequest},
};
use std::{
    error::Error,
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Notify;
use uuid::Uuid;

pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

pub struct Renderer {
    pub root: PathBuf,
    pub calls: AtomicUsize,
    pub hold: AtomicBool,
    pub panic_once: AtomicBool,
    pub cleanup_uncertain: AtomicBool,
    pub entered: Notify,
}

#[async_trait]
impl SpeechAudioAssemblyPort for Renderer {
    async fn assemble_speech(
        &self,
        request: SpeechAudioAssemblyRequest,
        cancel: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        assert!(
            !self.panic_once.swap(false, Ordering::SeqCst),
            "Deliberate fixture renderer death"
        );
        if self.cleanup_uncertain.load(Ordering::SeqCst) {
            return Err(PortError::CleanupFailed {
                resource: "ffmpeg_process".into(),
                message: "Fixture child exit is unconfirmed".into(),
            });
        }
        if self.hold.load(Ordering::SeqCst) {
            cancel.cancelled().await;
            return Err(PortError::Cancelled);
        }
        if cancel.is_cancelled() {
            return Err(PortError::Cancelled);
        }
        let stage = self.root.join(format!("audio-assembly-{}", Uuid::new_v4()));
        fs::create_dir(&stage).map_err(io)?;
        let input = fs::read(&request.source.aggregate_path).map_err(io)?;
        let (frames, start) = match request.mode {
            TimingMode::Continuous => (4800 + request.pause_after_ms[0] * 48, 0),
            TimingMode::SubtitleWindow => (request.source_duration_ms.unwrap_or(0) * 48, 4800),
        };
        let mut wave = source::wave()[..44].to_vec();
        wave[4..8].copy_from_slice(&(frames as u32 * 2 + 36).to_le_bytes());
        wave[40..44].copy_from_slice(&(frames as u32 * 2).to_le_bytes());
        wave.resize(44 + frames as usize * 2, 0);
        let offset = 44 + start as usize * 2;
        wave[offset..offset + 9600].copy_from_slice(&input[44..]);
        let wav_path = stage.join("assembled.wav");
        fs::write(&wav_path, &wave).map_err(io)?;
        let manifest_path = stage.join("assembly.json");
        let manifest = b"{\"fixture_renderer\":true}";
        fs::write(&manifest_path, manifest).map_err(io)?;
        Ok(StagedAudioAssembly {
            staging_root: stage,
            wav_path,
            manifest_path,
            wav_sha256: hash(&wave)?,
            manifest_sha256: hash(manifest)?,
            pcm_sha256: hash(&wave[44..])?,
            sample_rate: 48000,
            channels: 1,
            frames,
            peak_sample_abs: 200,
            full_scale_samples: 0,
            admission_guardian_pid: 0,
        })
    }

    async fn release_staged(&self, root: PathBuf) -> Result<(), PortError> {
        if root.parent() != Some(self.root.as_path())
            || !root
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("audio-assembly-"))
        {
            return Err(PortError::InvalidSource {
                message: "Fixture cleanup ownership mismatch".into(),
            });
        }
        for entry in fs::read_dir(&root).map_err(io)? {
            fs::remove_file(entry.map_err(io)?.path()).map_err(io)?;
        }
        fs::remove_dir(root).map_err(io)
    }
}

fn io(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}
fn hash(bytes: &[u8]) -> Result<Sha256Digest, PortError> {
    Sha256Digest::parse(source::digest(bytes)).map_err(|message| PortError::InvalidSource {
        message: message.into(),
    })
}

struct Events;
#[async_trait]
impl ports::events::AppEventPublisher for Events {
    async fn publish_project_updated(&self, _: &str) -> Result<(), PortError> {
        Ok(())
    }
    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), PortError> {
        Ok(())
    }
}

pub struct Fixture {
    pub source: AssemblySource,
    pub cases: AudioAssemblyUseCases,
    pub renderer: Arc<Renderer>,
    pub manager: Arc<jobs::manager::JobManager>,
}

impl Fixture {
    pub async fn new() -> TestResult<Self> {
        let source = AssemblySource::new().await?;
        let manager = Arc::new(jobs::manager::JobManager::new(
            Arc::new(SqliteJobRepository::new(source.pool.clone())),
            Arc::new(SqliteStorageUnitOfWork::new(source.pool.clone())),
            None,
        ));
        let root = source.directory.path().join("renderer");
        fs::create_dir(&root)?;
        let renderer = Arc::new(Renderer {
            root,
            calls: AtomicUsize::new(0),
            hold: AtomicBool::new(false),
            panic_once: AtomicBool::new(false),
            cleanup_uncertain: AtomicBool::new(false),
            entered: Notify::new(),
        });
        let cases = AudioAssemblyUseCases {
            projects: Arc::new(SqliteProjectRepository::new(source.pool.clone())),
            speech: source.speech.clone(),
            store: Arc::new(SqliteAudioAssemblyStore::new(source.pool.clone())),
            index: source.index.clone(),
            artifacts: source.files.clone(),
            inspector: Arc::new(ManagedSpeechAudioReader::new(
                source.directory.path().join("managed"),
                AudioAssemblyLimits::default(),
            )?),
            renderer: Some(renderer.clone()),
            renderer_error: None,
            ffmpeg_sha256: source::pin()?,
            runtime: manager.clone(),
            emitter: Arc::new(|_| {}),
        };
        Ok(Self {
            source,
            cases,
            renderer,
            manager,
        })
    }

    pub fn request(&self, pause: u64) -> InspectAssemblyRequest {
        InspectAssemblyRequest {
            source_run_id: self.source.source.run_id,
            settings: SpeechTimingSettings {
                mode: TimingMode::Continuous,
                source_duration_ms: None,
                pause_after_ms: vec![pause],
            },
        }
    }

    pub async fn prepare(&self, pause: u64) -> TestResult<AudioAssemblyView> {
        let request = self.request(pause);
        let preview = self
            .cases
            .inspect(
                self.source.project_id.clone(),
                request.clone(),
                CancellationToken::new(),
            )
            .await?;
        Ok(self
            .cases
            .prepare(
                self.source.project_id.clone(),
                PrepareAssemblyRequest {
                    source: request,
                    expected_preview_sha256: preview.preview_sha256,
                    reviewed: true,
                },
            )
            .await?)
    }

    pub async fn flush(&self) -> TestResult {
        OutboxWorker::new(
            SqliteOutboxRepository::new(self.source.pool.clone()),
            self.source.files.clone(),
            self.source.index.clone(),
            Arc::new(SqliteStorageUnitOfWork::new(self.source.pool.clone())),
            Arc::new(Events),
            Arc::new(LocalTempWorkspace::new(self.source.directory.path())),
            OutboxMaintenanceConfig::try_default()?,
        )
        .process_pending_messages(100)
        .await?;
        Ok(())
    }

    pub async fn publication(
        &self,
        id: Uuid,
    ) -> TestResult<ports::audio_jobs::AudioAssemblyPublication> {
        Ok(tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let Some(value) = self.cases.store.publication(id).await? {
                    return Ok::<_, PortError>(value);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await??)
    }

    pub async fn completed(&self, id: Uuid) -> TestResult<AudioAssemblyView> {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                self.flush().await?;
                let state = self.cases.state(self.source.project_id.clone(), id).await?;
                if state.result.is_some() {
                    return Ok::<_, Box<dyn Error>>(state);
                }
                if state
                    .job
                    .as_ref()
                    .is_some_and(|job| job.status == JobStatus::Failed)
                {
                    return Err(format!("Assembly failed: {:?}", state.job).into());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await?
    }

    pub async fn close(self) -> TestResult {
        let report = self.manager.drain_all(Duration::from_secs(5)).await?;
        assert_eq!(report.unconfirmed_count, 0);
        self.source.pool.close().await;
        Ok(())
    }
}
