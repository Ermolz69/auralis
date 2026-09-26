use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteJobRepository, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationHostJobStore, SqliteTranslationLinkStore,
        connect_sqlite,
    },
};
#[path = "support/translation_history_checks.rs"]
mod translation_history_checks;
use adapters_translate::{
    FormatInspector, TranslateResultEditor, TranslateResultReader, TranslateRunControl,
    TranslateRunExecutor, TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, EditTranslationResultRequest,
    EditTranslationResultUseCase, ExecuteTranslationRequest, ExecuteTranslationUseCase,
    GetTranslationComparisonRequest, GetTranslationComparisonUseCase,
    InspectTranslationSourceUseCase, PublishTranslationRequest, PublishTranslationResultUseCase,
    RecoverTranslationPublicationsUseCase, RunTranslationHostJobUseCase,
    SelectTranslationResultRequest, SelectTranslationResultUseCase, StartTranslationJobUseCase,
    TranslationRunRequest, TranslationRunUseCase,
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::job::JobStatus;
use domain::media::{ArtifactId, ArtifactKind, ArtifactState};
use domain::project::{Project, ProjectId};
use ports::artifact_index::ArtifactIndex;
use ports::repository::{JobRepository, ProjectRepository};
use ports::storage::ArtifactStore;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{
    StartTranslationRun, TranslationLinkStore, TranslationPublicationState,
};
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
    let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
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
    let host_job = StartTranslationJobUseCase::new(
        links.clone(),
        Arc::new(TranslateRunControl::new(translate_path.clone())),
        jobs.clone(),
    )
    .execute(run_request())
    .await?;
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
    let completed_job = SqliteJobRepository::new(pool.clone())
        .get(host_job.id())
        .await?
        .ok_or("missing host job")?;
    assert_eq!(completed_job.status(), &JobStatus::Completed);
    assert_eq!(completed_job.progress().processed_items, Some(1));
    assert_eq!(completed_job.progress().total_items, Some(1));
    let completed = run_control.status(run_request()).await?;
    assert_eq!(completed.phase, TranslationRunPhase::Validated);
    assert_eq!(completed.result_id, Some(result_id));
    assert_eq!(completed.needs_review, Some(true));
    let model_result_id = result_id;
    let editor = EditTranslationResultUseCase::new(
        inspector.clone(),
        links.clone(),
        Arc::new(TranslateResultEditor::new(translate_path.clone())),
    );
    let edited_result_id = Uuid::new_v4();
    let edit = || EditTranslationResultRequest {
        project_id: project.id().clone(),
        translation_id,
        base_result_id: result_id,
        result_id: edited_result_id,
        segment_id: 1,
        lines: vec!["Здравствуйте.".into()],
    };
    assert_eq!(editor.execute(edit()).await?, edited_result_id);
    assert_eq!(editor.execute(edit()).await?, edited_result_id);
    assert!(
        editor
            .execute(EditTranslationResultRequest {
                project_id: ProjectId::new(),
                ..edit()
            })
            .await
            .is_err()
    );
    let result_id = edited_result_id;
    let expected_output = "1\n00:00:01,000 --> 00:00:02,000\nЗдравствуйте.\n".as_bytes();
    let comparison = GetTranslationComparisonUseCase::new(
        inspector.clone(),
        links.clone(),
        Arc::new(TranslateResultReader::new(translate_path.clone())),
    );
    let compare = |result_id, offset, limit| GetTranslationComparisonRequest {
        project_id: project.id().clone(),
        translation_id,
        result_id,
        offset,
        limit,
    };
    let page = comparison.execute(compare(result_id, 0, 1)).await?;
    assert_eq!(page.total_segments, 1);
    assert_eq!(page.segments[0].source_lines, ["你好。"]);
    assert_eq!(page.segments[0].translated_lines, ["Здравствуйте."]);
    assert!(page.needs_review);
    assert!(
        comparison
            .execute(compare(result_id, 0, 101))
            .await
            .is_err()
    );
    assert!(
        comparison
            .execute(compare(result_id, 1, 1))
            .await?
            .segments
            .is_empty()
    );
    assert!(
        comparison
            .execute(GetTranslationComparisonRequest {
                project_id: ProjectId::new(),
                ..compare(result_id, 0, 1)
            })
            .await
            .is_err()
    );

    let publisher = PublishTranslationResultUseCase::new(
        inspector.clone(),
        links.clone(),
        Arc::new(TranslateResultReader::new(translate_path.clone())),
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
    let model_pending = publisher
        .execute(PublishTranslationRequest {
            project_id: project.id().clone(),
            translation_id,
            result_id: model_result_id,
        })
        .await?;
    assert_eq!(model_pending.result_revision, 1);
    let recovery = RecoverTranslationPublicationsUseCase::new(
        links.clone(),
        Arc::new(TranslateRunControl::new(translate_path.clone())),
        publisher.clone(),
    );
    let outcomes = recovery.execute().await?;
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].outcome.is_ok());
    let pending = links
        .get_publication(translation_id, result_id)
        .await?
        .ok_or("missing recovered edit publication")?;
    assert_eq!(pending.state, TranslationPublicationState::Pending);
    assert_eq!(pending.result_revision, 2);
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
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 2);
    let selected = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(selected.selected_result_id, Some(result_id));
    assert_eq!(
        links
            .get_publication(translation_id, model_result_id)
            .await?
            .ok_or("missing model publication")?
            .state,
        TranslationPublicationState::Pending
    );
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
    assert_eq!(std::fs::read(&output_path)?, expected_output);
    assert_eq!(std::fs::read(&source_path)?, source);
    assert!(String::from_utf8(expected_output.to_vec())?.contains("Здравствуйте."));

    let revised_result_id = Uuid::new_v4();
    assert_eq!(
        editor
            .execute(EditTranslationResultRequest {
                project_id: project.id().clone(),
                translation_id,
                base_result_id: edited_result_id,
                result_id: revised_result_id,
                segment_id: 1,
                lines: vec!["Добрый день.".into()],
            })
            .await?,
        revised_result_id
    );
    assert!(
        editor
            .execute(EditTranslationResultRequest {
                project_id: project.id().clone(),
                translation_id,
                base_result_id: model_result_id,
                result_id: Uuid::new_v4(),
                segment_id: 1,
                lines: vec!["Старая правка.".into()],
            })
            .await
            .is_err()
    );
    let revised = publisher
        .execute(PublishTranslationRequest {
            project_id: project.id().clone(),
            translation_id,
            result_id: revised_result_id,
        })
        .await?;
    assert!(revised.needs_review);
    assert_eq!(revised.result_revision, 3);
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let selected = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(selected.selected_result_id, Some(revised_result_id));
    let revised_artifact = artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::TranslatedTranscript)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == revised.artifact_id)
        .ok_or("missing revised artifact")?;
    assert_eq!(revised_artifact.state, ArtifactState::Ready);
    let revised_path = file_store.resolve_artifact(&revised_artifact).await?;
    assert!(String::from_utf8(std::fs::read(revised_path)?)?.contains("Добрый день."));
    assert_eq!(std::fs::read(&output_path)?, expected_output);
    assert_eq!(std::fs::read(&source_path)?, source);

    translation_history_checks::TranslationHistoryChecks {
        inspector: inspector.clone(),
        links: links.clone(),
        artifacts: artifacts.clone(),
        files: file_store.clone(),
        host_path: directory.path().join("auralis.sqlite"),
        translate_path: translate_path.clone(),
        source_path: source_path.clone(),
        output_path: output_path.clone(),
        project_id: project.id().clone(),
        translation_id,
        older_result_id: edited_result_id,
        latest_result_id: revised_result_id,
    }
    .verify()
    .await?;

    let recovered_result_id = Uuid::new_v4();
    editor
        .execute(EditTranslationResultRequest {
            project_id: project.id().clone(),
            translation_id,
            base_result_id: revised_result_id,
            result_id: recovered_result_id,
            segment_id: 1,
            lines: vec!["После перезапуска.".into()],
        })
        .await?;
    let outcomes = recovery.execute().await?;
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].outcome.is_ok());
    let recovered = links
        .get_publication(translation_id, recovered_result_id)
        .await?
        .ok_or("missing recovered selected-run edit")?;
    assert_eq!(recovered.result_revision, 4);
    assert_eq!(recovered.state, TranslationPublicationState::Pending);
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let selected = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(selected.selected_result_id, Some(recovered_result_id));
    assert_eq!(std::fs::read(&source_path)?, source);

    let stale_result_id = Uuid::new_v4();
    editor
        .execute(EditTranslationResultRequest {
            project_id: project.id().clone(),
            translation_id,
            base_result_id: recovered_result_id,
            result_id: stale_result_id,
            segment_id: 1,
            lines: vec!["Поздняя правка.".into()],
        })
        .await?;
    publisher
        .execute(PublishTranslationRequest {
            project_id: project.id().clone(),
            translation_id,
            result_id: stale_result_id,
        })
        .await?;
    let next_run_id = Uuid::new_v4();
    links
        .start_run(StartTranslationRun {
            translation_id,
            run_id: next_run_id,
            expected_active_run_id: None,
            expected_link_revision: selected.revision,
            updated_at: domain::chrono::Utc::now(),
        })
        .await?;
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let after_stale = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(after_stale.active_run_id, Some(next_run_id));
    assert_eq!(after_stale.selected_result_id, Some(recovered_result_id));
    let older_page = comparison.execute(compare(result_id, 0, 1)).await?;
    assert_eq!(older_page.run_id, run_id);
    assert_eq!(older_page.segments[0].translated_lines, ["Здравствуйте."]);
    let latest_page = comparison.execute(compare(revised_result_id, 0, 1)).await?;
    assert_eq!(latest_page.segments[0].translated_lines, ["Добрый день."]);
    let recovered_page = comparison
        .execute(compare(recovered_result_id, 0, 1))
        .await?;
    assert_eq!(
        recovered_page.segments[0].translated_lines,
        ["После перезапуска."]
    );
    assert_eq!(std::fs::read(&source_path)?, source);
    let chosen = SelectTranslationResultUseCase::new(
        inspector,
        links.clone(),
        Arc::new(TranslateResultReader::new(translate_path)),
        artifacts,
        file_store,
    )
    .execute(SelectTranslationResultRequest {
        project_id: project.id().clone(),
        translation_id,
        result_id: edited_result_id,
        expected_link_revision: after_stale.revision,
    })
    .await?;
    assert_eq!(chosen.active_run_id, Some(next_run_id));
    assert_eq!(chosen.selected_result_id, Some(edited_result_id));
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
