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
    ExecuteTranslationUseCase, GetTranslationComparisonRequest, GetTranslationComparisonUseCase,
    ImportTranslationSourceRequest, ImportTranslationSourceUseCase,
    InspectTranslationSourceUseCase, PublishTranslationRequest, PublishTranslationResultUseCase,
    RunTranslationHostJobUseCase, StartTranslationJobUseCase, TranslationRunRequest,
    TranslationRunUseCase,
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::media::{ArtifactKind, ArtifactState};
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationState};
use ports::translation_run_control::TranslationRunPhase;
use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);
const SOURCE: &[u8] = b"WEBVTT\r\n\r\nNOTE provenance\r\nsynthetic\r\n\r\n00:01.000 --> 00:02.000\r\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\r\n";
const OUTPUT: &[u8] =
    "WEBVTT\r\n\r\nNOTE provenance\r\nsynthetic\r\n\r\n00:01.000 --> 00:02.000\r\nПривет.\r\n"
        .as_bytes();

struct NoEvents;

#[async_trait::async_trait]
impl ports::events::AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), ports::PortError> {
        Err(ports::PortError::Unexpected {
            message: "unexpected project event".into(),
        })
    }

    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), ports::PortError> {
        Err(ports::PortError::Unexpected {
            message: "unexpected transcript event".into(),
        })
    }
}

#[tokio::test]
async fn webvtt_runs_through_both_databases_and_publishes_a_separate_verified_copy()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
    let store = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
    let project = projects
        .create(Project::new("WebVTT translation".into())?)
        .await?;
    let external_source = directory.path().join("incoming.vtt");
    std::fs::write(&external_source, SOURCE)?;
    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        store.clone(),
        artifacts.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(directory.path())),
        OutboxMaintenanceConfig::try_default()?,
    )
    .with_translations(links.clone());
    let source_artifact = ImportTranslationSourceUseCase::new(
        projects.clone(),
        store.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool)),
        Arc::new(FormatInspector),
    )
    .execute(ImportTranslationSourceRequest {
        project_id: project.id().clone(),
        path: external_source.clone(),
        format: TranslationSourceFormat::Vtt,
    })
    .await?;
    assert_eq!(source_artifact.state, ArtifactState::PendingFinalize);
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let managed_source = artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::OriginalSubtitle)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == source_artifact.id)
        .ok_or("missing managed original")?;
    assert_eq!(managed_source.state, ArtifactState::Ready);
    let source_path = store.resolve_artifact(&managed_source).await?;
    assert_eq!(std::fs::read(&source_path)?, SOURCE);
    assert_eq!(std::fs::read(&external_source)?, SOURCE);

    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects,
        artifacts.clone(),
        store.clone(),
        Arc::new(FormatInspector),
    ));
    let translate_path = directory.path().join("translate.sqlite");
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    BeginTranslationUseCase::new(
        inspector.clone(),
        links.clone(),
        Arc::new(TranslateRunRegistrar::new(translate_path.clone())),
    )
    .execute(BeginTranslationRequest {
        project_id: project.id().clone(),
        source_artifact_id: managed_source.id,
        format: TranslationSourceFormat::Vtt,
        source_language: "zh".into(),
        translation_id,
        run_id,
        profile_manifest_json: PROFILE.into(),
        created_at: domain::chrono::Utc::now(),
    })
    .await?;
    assert_eq!(
        links
            .get(translation_id)
            .await?
            .ok_or("missing link")?
            .active_run_id,
        Some(run_id)
    );

    let run_control = TranslationRunUseCase::new(
        links.clone(),
        Arc::new(TranslateRunControl::new(translate_path.clone())),
    );
    let run_request = TranslationRunRequest {
        project_id: project.id().clone(),
        translation_id,
    };
    let host_job = StartTranslationJobUseCase::new(
        links.clone(),
        Arc::new(TranslateRunControl::new(translate_path.clone())),
        jobs.clone(),
    )
    .execute(run_request.clone())
    .await?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/", listener.local_addr()?);
    let server = std::thread::spawn(move || respond_once(listener));
    let executor = Arc::new(ExecuteTranslationUseCase::new(
        inspector.clone(),
        links.clone(),
        Arc::new(TranslateRunRegistrar::new(translate_path.clone())),
        Arc::new(TranslateRunExecutor::experimental_unchecked(
            translate_path.clone(),
        )),
    ));
    let result_id = RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), executor)
        .execute(
            ExecuteTranslationRequest {
                project_id: project.id().clone(),
                translation_id,
                server_url: endpoint,
                start_guard: None,
            },
            host_job.id(),
        )
        .await?;
    server
        .join()
        .map_err(|_| "mock server panicked")?
        .map_err(|error| error.to_string())?;
    assert!(jobs.active_for_run(run_id).await?.is_none());
    let status = run_control.status(run_request).await?;
    assert_eq!(status.phase, TranslationRunPhase::Validated);
    assert_eq!(status.result_id, Some(result_id));
    assert_eq!(status.needs_review, Some(true));

    let comparison = GetTranslationComparisonUseCase::new(
        inspector.clone(),
        links.clone(),
        Arc::new(TranslateResultReader::new(translate_path.clone())),
    );
    let page = comparison
        .execute(GetTranslationComparisonRequest {
            project_id: project.id().clone(),
            translation_id,
            result_id,
            offset: 0,
            limit: 1,
        })
        .await?;
    assert_eq!(page.total_segments, 1);
    assert_eq!(page.segments[0].source_lines, ["你好。"]);
    assert_eq!(page.segments[0].translated_lines, ["Привет."]);

    let pending = PublishTranslationResultUseCase::new(
        inspector,
        links.clone(),
        Arc::new(TranslateResultReader::new(translate_path)),
        store.clone(),
    )
    .execute(PublishTranslationRequest {
        project_id: project.id().clone(),
        translation_id,
        result_id,
    })
    .await?;
    assert_eq!(pending.state, TranslationPublicationState::Pending);
    assert!(pending.needs_review);
    assert_eq!(
        links
            .get(translation_id)
            .await?
            .ok_or("missing link")?
            .selected_result_id,
        None
    );
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let selected = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(selected.selected_result_id, Some(result_id));
    assert_eq!(
        selected.output_artifact_id,
        Some(pending.artifact_id.clone())
    );
    let output_artifact = artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::TranslatedTranscript)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == pending.artifact_id)
        .ok_or("missing translated artifact")?;
    assert_eq!(output_artifact.state, ArtifactState::Ready);
    let output_path = store.resolve_artifact(&output_artifact).await?;
    assert_ne!(output_path, source_path);
    assert_eq!(std::fs::read(output_path)?, OUTPUT);
    assert_eq!(std::fs::read(source_path)?, SOURCE);
    assert_eq!(std::fs::read(external_source)?, SOURCE);
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
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    read_request(&mut stream)?;
    let body = r#"{"choices":[{"message":{"content":"Привет."},"finish_reason":"stop"}]}"#;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err("model request ended early".into());
        }
        raw.extend_from_slice(&buffer[..count]);
        if let Some(header_end) = raw.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&raw[..header_end])?;
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .ok_or("missing model request length")?;
            if raw.len() >= header_end + 4 + length {
                return Ok(());
            }
        }
    }
}
