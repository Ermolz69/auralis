use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationHostJobStore, SqliteTranslationLinkStore,
        connect_sqlite,
    },
};
use adapters_translate::{
    FormatInspector, ManagedLlamaRuntime, ManagedRuntimeConfig, TranslateResultReader,
    TranslateRunControl, TranslateRunExecutor, TranslateRunRecovery, TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ExecuteTranslationUseCase,
    InspectTranslationSourceUseCase, PublishTranslationResultUseCase,
    RecoverTranslationHostJobsUseCase, RunManagedTranslationUseCase, RunTranslationHostJobUseCase,
    StartTranslationJobUseCase, TranslationRunRequest,
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use auralis_translation::{RunId, RunState};
use auralis_translation_sqlite::{SqliteConfig, TranslateDb};
use domain::media::{ArtifactId, ArtifactKind, ArtifactState};
use domain::project::{Project, ProjectId};
use ports::artifact_index::ArtifactIndex;
use ports::error::PortError;
use ports::events::AppEventPublisher;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{
    TranslationLinkStore, TranslationPublicationState, TranslationRunIntent,
};
use ports::translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort};
use sqlx::SqlitePool;
use std::error::Error;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
);
const SOURCE: &[u8] = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n\n2\n00:00:03,000 --> 00:00:04,000\n\xe5\x86\x8d\xe8\xa7\x81\xe3\x80\x82\n";
const ROOT_ENV: &str = "AURALIS_MANAGED_CRASH_ROOT";
const PROJECT_ENV: &str = "AURALIS_MANAGED_CRASH_PROJECT";
const TRANSLATION_ENV: &str = "AURALIS_MANAGED_CRASH_TRANSLATION";
const SERVER_ENV: &str = "AURALIS_TEST_LLAMA_SERVER";
const MODEL_ENV: &str = "AURALIS_TEST_GGUF";
const CHECKPOINT_TIMEOUT: Duration = Duration::from_secs(420);
const SERVER_EXIT_TIMEOUT: Duration = Duration::from_secs(20);

struct NoEvents;

#[async_trait::async_trait]
impl AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), PortError> {
        Ok(())
    }

    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), PortError> {
        Ok(())
    }
}

struct EndpointRecordingRuntime {
    inner: ManagedLlamaRuntime,
    endpoint_file: PathBuf,
}

#[async_trait::async_trait]
impl TranslationModelRuntimePort for EndpointRecordingRuntime {
    async fn acquire(
        &self,
        intent: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        let lease = self.inner.acquire(intent).await?;
        std::fs::write(&self.endpoint_file, lease.server_url()).map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        Ok(lease)
    }
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Services {
    managed: RunManagedTranslationUseCase,
    links: Arc<SqliteTranslationLinkStore>,
    jobs: Arc<SqliteTranslationHostJobStore>,
    artifacts: Arc<SqliteArtifactIndex>,
    files: Arc<LocalArtifactStore>,
}

fn services(
    root: &Path,
    pool: &SqlitePool,
    runtime: Arc<dyn TranslationModelRuntimePort>,
) -> Services {
    let translate_path = root.join("translate.sqlite");
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
    let files = Arc::new(LocalArtifactStore::new(root.join("artifacts")));
    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects,
        artifacts.clone(),
        files.clone(),
        Arc::new(FormatInspector),
    ));
    let registrar = Arc::new(TranslateRunRegistrar::new(translate_path.clone()));
    let control = Arc::new(TranslateRunControl::new(translate_path.clone()));
    let executor = Arc::new(ExecuteTranslationUseCase::new(
        inspector.clone(),
        links.clone(),
        registrar,
        Arc::new(TranslateRunExecutor::new(translate_path.clone())),
    ));
    let managed = RunManagedTranslationUseCase::new(
        links.clone(),
        runtime,
        control.clone(),
        StartTranslationJobUseCase::new(links.clone(), control, jobs.clone()),
        RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), executor),
        PublishTranslationResultUseCase::new(
            inspector,
            links.clone(),
            Arc::new(TranslateResultReader::new(translate_path)),
            files.clone(),
        ),
    );
    Services {
        managed,
        links,
        jobs,
        artifacts,
        files,
    }
}

fn runtime(executable: PathBuf, model_file: PathBuf) -> Result<ManagedLlamaRuntime, PortError> {
    ManagedLlamaRuntime::new(ManagedRuntimeConfig {
        executable,
        model_file,
        context_tokens: 2048,
        gpu_layers: 99,
        startup_timeout: Duration::from_secs(120),
        readiness_interval: Duration::from_millis(250),
    })
}

#[tokio::test]
#[ignore = "requires the pinned checked llama.cpp binary and GGUF"]
async fn managed_crash_child() -> Result<(), Box<dyn Error>> {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    let project_id: ProjectId = std::env::var(PROJECT_ENV)?.parse()?;
    let translation_id = Uuid::parse_str(&std::env::var(TRANSLATION_ENV)?)?;
    let executable = PathBuf::from(std::env::var_os(SERVER_ENV).ok_or(SERVER_ENV)?);
    let model_file = PathBuf::from(std::env::var_os(MODEL_ENV).ok_or(MODEL_ENV)?);
    let pool = connect_sqlite(root.join("auralis.sqlite")).await?;
    let runtime = Arc::new(EndpointRecordingRuntime {
        inner: runtime(executable, model_file)?,
        endpoint_file: root.join("managed-endpoint"),
    });
    let app = services(&root, &pool, runtime);
    app.managed
        .execute(TranslationRunRequest {
            project_id,
            translation_id,
        })
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires the pinned checked llama.cpp binary and GGUF"]
async fn killed_managed_worker_releases_model_and_resumes_remaining_block()
-> Result<(), Box<dyn Error>> {
    let executable = PathBuf::from(std::env::var_os(SERVER_ENV).ok_or(SERVER_ENV)?);
    let model_file = PathBuf::from(std::env::var_os(MODEL_ENV).ok_or(MODEL_ENV)?);
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let pool = connect_sqlite(root.join("auralis.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let files = Arc::new(LocalArtifactStore::new(root.join("artifacts")));
    let project = projects
        .create(Project::new("Managed crash".into())?)
        .await?;
    let source_id = ArtifactId::new();
    let source_key = format!("projects/{}/original.srt", project.id());
    let source_path = files.resolve_storage_key(&source_key)?;
    std::fs::create_dir_all(source_path.parent().ok_or("missing parent")?)?;
    std::fs::write(&source_path, SOURCE)?;
    let created_at = domain::chrono::Utc::now();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, ?, 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&source_key)
        .bind(i64::try_from(SOURCE.len())?)
        .bind(created_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .execute(&pool)
        .await?;
    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects,
        artifacts,
        files,
        Arc::new(FormatInspector),
    ));
    let mut profile: serde_json::Value = serde_json::from_str(PROFILE)?;
    profile["target_segments_per_block"] = serde_json::json!(1);
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    BeginTranslationUseCase::new(
        inspector,
        links,
        Arc::new(TranslateRunRegistrar::new(root.join("translate.sqlite"))),
    )
    .execute(BeginTranslationRequest {
        project_id: project.id().clone(),
        source_artifact_id: source_id,
        format: TranslationSourceFormat::Srt,
        source_language: "zh".into(),
        translation_id,
        run_id,
        profile_manifest_json: serde_json::to_string(&profile)?,
        created_at,
    })
    .await?;
    pool.close().await;

    let mut child = KillOnDrop(
        Command::new(std::env::current_exe()?)
            .args(["--exact", "managed_crash_child", "--ignored", "--nocapture"])
            .env(ROOT_ENV, root)
            .env(PROJECT_ENV, project.id().to_string())
            .env(TRANSLATION_ENV, translation_id.to_string())
            .env(SERVER_ENV, &executable)
            .env(MODEL_ENV, &model_file)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?,
    );
    let translate_path = root.join("translate.sqlite");
    let run = RunId::new(run_id).ok_or("invalid run ID")?;
    let started = Instant::now();
    loop {
        if child.0.try_wait()?.is_some() {
            return Err("managed worker exited before first checkpoint".into());
        }
        let database = TranslateDb::open(&translate_path, SqliteConfig::default())?;
        if !database.checkpoints(run)?.is_empty() {
            assert_eq!(database.checkpoints(run)?.len(), 1);
            assert_eq!(database.run_state(run)?, RunState::Running);
            break;
        }
        if started.elapsed() > CHECKPOINT_TIMEOUT {
            return Err("managed first checkpoint timed out".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let endpoint = std::fs::read_to_string(root.join("managed-endpoint"))?;
    let address = endpoint
        .strip_prefix("http://")
        .and_then(|value| value.strip_suffix('/'))
        .ok_or("unexpected managed endpoint")?
        .to_owned();
    assert!(TcpStream::connect(&address).is_ok());
    child.0.kill()?;
    assert!(!child.0.wait()?.success());
    let stopped = Instant::now();
    while TcpStream::connect(&address).is_ok() {
        if stopped.elapsed() > SERVER_EXIT_TIMEOUT {
            return Err("managed model child remained reachable after worker kill".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let pool = connect_sqlite(root.join("auralis.sqlite")).await?;
    let runtime = Arc::new(runtime(executable, model_file)?);
    let app = services(root, &pool, runtime);
    let recovery = RecoverTranslationHostJobsUseCase::new(
        app.jobs.clone(),
        Arc::new(TranslateRunRecovery::new(translate_path.clone())),
    );
    let recovered = recovery.execute().await?;
    assert_eq!(recovered.len(), 1);
    assert!(recovered[0].outcome.is_ok());
    assert!(recovery.execute().await?.is_empty());
    assert!(app.jobs.active_for_run(run_id).await?.is_none());
    let link = app.links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(link.active_run_id, Some(run_id));
    assert_eq!(link.selected_result_id, None);
    let database = TranslateDb::open(&translate_path, SqliteConfig::default())?;
    assert_eq!(database.run_state(run)?, RunState::Paused);
    assert_eq!(database.checkpoints(run)?.len(), 1);
    drop(database);

    let publication = app
        .managed
        .execute(TranslationRunRequest {
            project_id: project.id().clone(),
            translation_id,
        })
        .await?;
    assert_eq!(publication.state, TranslationPublicationState::Pending);
    let database = TranslateDb::open(&translate_path, SqliteConfig::default())?;
    assert_eq!(database.checkpoints(run)?.len(), 2);
    drop(database);
    let outbox = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        app.files.clone(),
        app.artifacts.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool)),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root)),
        OutboxMaintenanceConfig::try_default()?,
    )
    .with_translations(app.links.clone());
    assert_eq!(outbox.process_pending_messages(10).await?.fetched, 1);
    let link = app
        .links
        .get(translation_id)
        .await?
        .ok_or("missing ready link")?;
    assert_eq!(link.selected_result_id, Some(publication.result_id));
    let output = app
        .artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::TranslatedTranscript)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == publication.artifact_id)
        .ok_or("missing output artifact")?;
    assert_eq!(output.state, ArtifactState::Ready);
    let output_path = app.files.resolve_artifact(&output).await?;
    let translated = std::fs::read(output_path)?;
    assert_ne!(translated, SOURCE);
    assert_eq!(std::fs::read(source_path)?, SOURCE);
    let text = String::from_utf8(translated)?;
    assert!(text.contains("00:00:01,000 --> 00:00:02,000"));
    assert!(text.contains("00:00:03,000 --> 00:00:04,000"));
    Ok(())
}
