use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationLinkStore, connect_sqlite,
    },
};
use adapters_translate::{
    FormatInspector, TranslateResultReader, TranslateRunControl, TranslateRunExecutor,
    TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ExecuteTranslationRequest,
    ExecuteTranslationUseCase, InspectTranslationSourceUseCase, PublishTranslationRequest,
    PublishTranslationResultUseCase, TranslationRunRequest, TranslationRunUseCase,
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::media::{ArtifactId, ArtifactKind, ArtifactState};
use domain::project::{Project, ProjectId};
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
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

struct NoEvents;

#[async_trait::async_trait]
impl ports::events::AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), ports::PortError> {
        Err(ports::PortError::Unexpected {
            message: "unexpected event".into(),
        })
    }

    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), ports::PortError> {
        Err(ports::PortError::Unexpected {
            message: "unexpected event".into(),
        })
    }
}

#[tokio::test]
async fn verified_result_publishes_as_separate_ready_artifact_with_review_flag()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let file_store = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
    let project = projects.create(Project::new("Translation".into())?).await?;
    let source = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
    let source_id = ArtifactId::new();
    let key = format!("projects/{}/original.srt", project.id());
    let source_path = file_store.resolve_storage_key(&key)?;
    std::fs::create_dir_all(source_path.parent().ok_or("missing source parent")?)?;
    std::fs::write(&source_path, source)?;
    let created_at = domain::chrono::Utc::now();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, ?, 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&key)
        .bind(i64::try_from(source.len())?)
        .bind(created_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .execute(&pool).await?;
    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects,
        artifacts.clone(),
        file_store.clone(),
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
        source_artifact_id: source_id,
        format: TranslationSourceFormat::Srt,
        source_language: "zh".into(),
        translation_id,
        run_id,
        profile_manifest_json: PROFILE.into(),
        created_at,
    })
    .await?;

    let run_control = TranslationRunUseCase::new(
        links.clone(),
        Arc::new(TranslateRunControl::new(translate_path.clone())),
    );
    let run_request = || TranslationRunRequest {
        project_id: project.id().clone(),
        translation_id,
    };
    assert!(
        run_control
            .pause(TranslationRunRequest {
                project_id: ProjectId::new(),
                translation_id,
            })
            .await
            .is_err()
    );
    run_control.pause(run_request()).await?;
    assert_eq!(
        run_control.status(run_request()).await?.phase,
        TranslationRunPhase::Paused
    );

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/", listener.local_addr()?);
    let server = std::thread::spawn(move || respond_once(listener));
    let result_id = ExecuteTranslationUseCase::new(
        inspector.clone(),
        links.clone(),
        Arc::new(TranslateRunRegistrar::new(translate_path.clone())),
        Arc::new(TranslateRunExecutor::experimental_unchecked(
            translate_path.clone(),
        )),
    )
    .execute(ExecuteTranslationRequest {
        project_id: project.id().clone(),
        translation_id,
        server_url: endpoint,
    })
    .await?;
    server
        .join()
        .map_err(|_| "mock server panicked")?
        .map_err(|error| error.to_string())?;
    let completed = run_control.status(run_request()).await?;
    assert_eq!(completed.phase, TranslationRunPhase::Validated);
    assert_eq!(completed.result_id, Some(result_id));
    assert_eq!(completed.needs_review, Some(true));
    let expected_output =
        b"1\n00:00:01,000 --> 00:00:02,000\n\xd0\x9f\xd1\x80\xd0\xb8\xd0\xb2\xd0\xb5\xd1\x82.\n";

    let publisher = PublishTranslationResultUseCase::new(
        inspector,
        links.clone(),
        Arc::new(TranslateResultReader::new(translate_path)),
        file_store.clone(),
    );
    let absent_result_id = Uuid::new_v4();
    assert!(
        publisher
            .execute(PublishTranslationRequest {
                project_id: project.id().clone(),
                translation_id,
                result_id: absent_result_id,
            })
            .await
            .is_err()
    );
    assert!(
        links
            .get_publication(translation_id, absent_result_id)
            .await?
            .is_none()
    );
    let request = || PublishTranslationRequest {
        project_id: project.id().clone(),
        translation_id,
        result_id,
    };
    let pending = publisher.execute(request()).await?;
    assert_eq!(pending.state, TranslationPublicationState::Pending);
    assert!(pending.needs_review);
    let repeated = publisher.execute(request()).await?;
    assert_eq!(pending.artifact_id, repeated.artifact_id);
    assert_eq!(std::fs::read(&source_path)?, source);

    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        file_store.clone(),
        artifacts.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(directory.path())),
        OutboxMaintenanceConfig::try_default()?,
    )
    .with_translations(links.clone());
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let selected = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(selected.selected_result_id, Some(result_id));
    assert_eq!(
        selected.output_artifact_id,
        Some(pending.artifact_id.clone())
    );
    let repeated_ready = publisher.execute(request()).await?;
    assert_eq!(repeated_ready.state, TranslationPublicationState::Ready);
    assert_eq!(repeated_ready.artifact_id, pending.artifact_id);
    let output_artifact = artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::TranslatedTranscript)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == pending.artifact_id)
        .ok_or("missing output artifact")?;
    assert_eq!(output_artifact.state, ArtifactState::Ready);
    let output_path = file_store.resolve_artifact(&output_artifact).await?;
    assert_eq!(std::fs::read(output_path)?, expected_output);
    assert_eq!(std::fs::read(&source_path)?, source);
    assert!(String::from_utf8(expected_output.to_vec())?.contains("Привет."));
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
