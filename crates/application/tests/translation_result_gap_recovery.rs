use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationHostJobStore, SqliteTranslationLinkStore,
        connect_sqlite,
    },
};
use adapters_translate::{
    FormatInspector, TranslateResultReader, TranslateRunControl, TranslateRunExecutor,
    TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ExecuteTranslationRequest,
    ExecuteTranslationUseCase, InspectTranslationSourceUseCase, PublishTranslationResultUseCase,
    RecoverTranslationPublicationsUseCase, RunTranslationHostJobUseCase,
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
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationState};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);
const SOURCE: &[u8] = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
const ROOT_ENV: &str = "AURALIS_RESULT_GAP_ROOT";
const PROJECT_ENV: &str = "AURALIS_RESULT_GAP_PROJECT";
const TRANSLATION_ENV: &str = "AURALIS_RESULT_GAP_TRANSLATION";
const MARKER_TIMEOUT: Duration = Duration::from_secs(30);

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct NoEvents;

#[async_trait::async_trait]
impl AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), PortError> {
        Err(PortError::Unexpected {
            message: "unexpected project event".into(),
        })
    }

    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), PortError> {
        Err(PortError::Unexpected {
            message: "unexpected transcript event".into(),
        })
    }
}

fn inspector(
    pool: &sqlx::SqlitePool,
    files: Arc<LocalArtifactStore>,
) -> Arc<InspectTranslationSourceUseCase> {
    Arc::new(InspectTranslationSourceUseCase::new(
        Arc::new(SqliteProjectRepository::new(pool.clone())),
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
        files,
        Arc::new(FormatInspector),
    ))
}

#[tokio::test]
async fn result_gap_child() -> Result<(), Box<dyn Error>> {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    let project_id: ProjectId = std::env::var(PROJECT_ENV)?.parse()?;
    let translation_id = Uuid::parse_str(&std::env::var(TRANSLATION_ENV)?)?;
    let pool = connect_sqlite(root.join("auralis.sqlite")).await?;
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
    let translate_path = root.join("translate.sqlite");
    let inspect = inspector(
        &pool,
        Arc::new(LocalArtifactStore::new(root.join("artifacts"))),
    );
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/", listener.local_addr()?);
    let server = std::thread::spawn(move || respond_once(listener));
    let request = TranslationRunRequest {
        project_id: project_id.clone(),
        translation_id,
    };
    let host_job = StartTranslationJobUseCase::new(
        links.clone(),
        Arc::new(TranslateRunControl::new(translate_path.clone())),
        jobs.clone(),
    )
    .execute(request)
    .await?;
    let executor = Arc::new(ExecuteTranslationUseCase::new(
        inspect,
        links.clone(),
        Arc::new(TranslateRunRegistrar::new(translate_path.clone())),
        Arc::new(TranslateRunExecutor::experimental_unchecked(translate_path)),
    ));
    let result_id = RunTranslationHostJobUseCase::new(links, jobs, executor)
        .execute(
            ExecuteTranslationRequest {
                project_id,
                translation_id,
                server_url: endpoint,
            },
            host_job.id(),
        )
        .await?;
    server
        .join()
        .map_err(|_| "mock server panicked")?
        .map_err(|error| error.to_string())?;
    std::fs::write(root.join("validated-result"), result_id.to_string())?;
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}

#[tokio::test]
async fn validated_result_is_published_after_worker_kill_and_two_restarts()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let host_path = root.join("auralis.sqlite");
    let translate_path = root.join("translate.sqlite");
    let pool = connect_sqlite(&host_path).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let files = Arc::new(LocalArtifactStore::new(root.join("artifacts")));
    let project = projects.create(Project::new("Result gap".into())?).await?;
    let source_id = ArtifactId::new();
    let source_key = format!("projects/{}/original.srt", project.id());
    let source_path = files.resolve_storage_key(&source_key)?;
    std::fs::create_dir_all(source_path.parent().ok_or("missing source parent")?)?;
    std::fs::write(&source_path, SOURCE)?;
    let created_at = domain::chrono::Utc::now();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, ?, 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&source_key)
        .bind(i64::try_from(SOURCE.len())?)
        .bind(created_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .execute(&pool).await?;
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    BeginTranslationUseCase::new(
        inspector(&pool, files),
        links,
        Arc::new(TranslateRunRegistrar::new(translate_path.clone())),
    )
    .execute(BeginTranslationRequest {
        project_id: project.id().clone(),
        source_artifact_id: source_id,
        format: TranslationSourceFormat::Srt,
        source_language: "zh".into(),
        translation_id,
        run_id,
        profile_manifest_json: PROFILE.into(),
        created_at,
    })
    .await?;
    pool.close().await;

    let mut child = KillOnDrop(
        Command::new(std::env::current_exe()?)
            .args(["--exact", "result_gap_child", "--nocapture"])
            .env(ROOT_ENV, root)
            .env(PROJECT_ENV, project.id().to_string())
            .env(TRANSLATION_ENV, translation_id.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?,
    );
    let marker = root.join("validated-result");
    let started = Instant::now();
    while !marker.exists() {
        if child.0.try_wait()?.is_some() {
            return Err("worker exited before Translate result commit".into());
        }
        if started.elapsed() > MARKER_TIMEOUT {
            return Err("Translate result commit timed out".into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let result_id = Uuid::parse_str(&std::fs::read_to_string(marker)?)?;
    let run = RunId::new(run_id).ok_or("invalid run ID")?;
    assert_eq!(
        TranslateDb::open(&translate_path, SqliteConfig::default())?.run_state(run)?,
        RunState::Validated
    );
    child.0.kill()?;
    assert!(!child.0.wait()?.success());

    let pool = connect_sqlite(&host_path).await?;
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let files = Arc::new(LocalArtifactStore::new(root.join("artifacts")));
    assert_eq!(
        links
            .get(translation_id)
            .await?
            .ok_or("missing link")?
            .selected_result_id,
        None
    );
    let recovery = RecoverTranslationPublicationsUseCase::new(
        links.clone(),
        Arc::new(TranslateRunControl::new(translate_path.clone())),
        PublishTranslationResultUseCase::new(
            inspector(&pool, files.clone()),
            links.clone(),
            Arc::new(TranslateResultReader::new(translate_path)),
            files.clone(),
        ),
    );
    let outcomes = recovery.execute().await?;
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].outcome.is_ok());
    assert!(recovery.execute().await?[0].outcome.is_ok());
    let publication = links
        .get_publication(translation_id, result_id)
        .await?
        .ok_or("missing pending publication")?;
    assert_eq!(publication.state, TranslationPublicationState::Pending);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM translation_publications")
            .fetch_one(&pool)
            .await?,
        1
    );
    assert_eq!(
        links
            .get(translation_id)
            .await?
            .ok_or("missing link")?
            .selected_result_id,
        None
    );
    pool.close().await;

    let pool = connect_sqlite(&host_path).await?;
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let outbox = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        files.clone(),
        artifacts.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root)),
        OutboxMaintenanceConfig::try_default()?,
    )
    .with_translations(links.clone());
    assert_eq!(outbox.process_pending_messages(10).await?.fetched, 1);
    assert_eq!(outbox.process_pending_messages(10).await?.fetched, 0);
    let link = links
        .get(translation_id)
        .await?
        .ok_or("missing ready link")?;
    assert_eq!(link.selected_result_id, Some(result_id));
    assert_eq!(link.active_run_id, None);
    let output = artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::TranslatedTranscript)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == publication.artifact_id)
        .ok_or("missing output artifact")?;
    assert_eq!(output.state, ArtifactState::Ready);
    let output_path = files.resolve_artifact(&output).await?;
    assert_ne!(std::fs::read(output_path)?, SOURCE);
    assert_eq!(std::fs::read(source_path)?, SOURCE);
    Ok(())
}

fn respond_once(listener: TcpListener) -> Result<(), Box<dyn Error + Send + Sync>> {
    listener.set_nonblocking(true)?;
    let started = Instant::now();
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if started.elapsed() > Duration::from_secs(10) {
                    return Err("timed out waiting for model request".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    };
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err("model request ended early".into());
        }
        request.extend_from_slice(&buffer[..count]);
        if let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&request[..header_end])?;
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .ok_or("missing model request length")?;
            if request.len() >= header_end + 4 + length {
                break;
            }
        }
    }
    let body = r#"{"choices":[{"message":{"content":"Привет."},"finish_reason":"stop"}]}"#;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}
