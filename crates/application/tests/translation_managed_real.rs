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
    TranslateRunControl, TranslateRunExecutor, TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ExecuteTranslationUseCase,
    InspectTranslationSourceUseCase, PublishTranslationResultUseCase, RunManagedTranslationUseCase,
    RunTranslationHostJobUseCase, StartTranslationJobUseCase, TranslationRunRequest,
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::media::{ArtifactId, ArtifactKind, ArtifactState};
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::error::PortError;
use ports::events::AppEventPublisher;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationState};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
);
const SOURCE: &[u8] = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
const SERVER_ENV: &str = "AURALIS_TEST_LLAMA_SERVER";
const MODEL_ENV: &str = "AURALIS_TEST_GGUF";

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

#[tokio::test]
#[ignore = "requires an explicitly installed checked llama.cpp binary and GGUF"]
async fn managed_model_translates_and_publishes_separate_project_copy() -> Result<(), Box<dyn Error>>
{
    let executable = PathBuf::from(std::env::var_os(SERVER_ENV).ok_or(SERVER_ENV)?);
    let model_file = PathBuf::from(std::env::var_os(MODEL_ENV).ok_or(MODEL_ENV)?);
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
    let files = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
    let project = projects
        .create(Project::new("Managed real translation".into())?)
        .await?;
    let source_id = ArtifactId::new();
    let storage_key = format!("projects/{}/original.srt", project.id());
    let source_path = files.resolve_storage_key(&storage_key)?;
    std::fs::create_dir_all(source_path.parent().ok_or("missing source parent")?)?;
    std::fs::write(&source_path, SOURCE)?;
    let created_at = domain::chrono::Utc::now();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, ?, 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&storage_key)
        .bind(i64::try_from(SOURCE.len())?)
        .bind(created_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .execute(&pool)
        .await?;
    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects,
        artifacts.clone(),
        files.clone(),
        Arc::new(FormatInspector),
    ));
    let translate_path = directory.path().join("translate.sqlite");
    let registrar = Arc::new(TranslateRunRegistrar::new(translate_path.clone()));
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    BeginTranslationUseCase::new(inspector.clone(), links.clone(), registrar.clone())
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
    let control = Arc::new(TranslateRunControl::new(translate_path.clone()));
    let executor = Arc::new(ExecuteTranslationUseCase::new(
        inspector.clone(),
        links.clone(),
        registrar,
        Arc::new(TranslateRunExecutor::new(translate_path.clone())),
    ));
    let runtime = Arc::new(ManagedLlamaRuntime::new(ManagedRuntimeConfig {
        executable,
        model_file,
        context_tokens: 2048,
        gpu_layers: 99,
        startup_timeout: Duration::from_secs(120),
        readiness_interval: Duration::from_millis(250),
    })?);
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
    let publication = managed
        .execute(TranslationRunRequest {
            project_id: project.id().clone(),
            translation_id,
        })
        .await?;
    assert_eq!(publication.state, TranslationPublicationState::Pending);
    assert!(publication.needs_review);
    assert!(jobs.active_for_run(run_id).await?.is_none());
    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        files.clone(),
        artifacts.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool)),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(directory.path())),
        OutboxMaintenanceConfig::try_default()?,
    )
    .with_translations(links.clone());
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let link = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(link.selected_result_id, Some(publication.result_id));
    assert_eq!(
        link.output_artifact_id,
        Some(publication.artifact_id.clone())
    );
    let output = artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::TranslatedTranscript)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == publication.artifact_id)
        .ok_or("missing translated artifact")?;
    assert_eq!(output.state, ArtifactState::Ready);
    let output_path = files.resolve_artifact(&output).await?;
    let translated = std::fs::read(output_path)?;
    assert_ne!(translated, SOURCE);
    assert_eq!(std::fs::read(&source_path)?, SOURCE);
    let text = String::from_utf8(translated)?;
    assert!(text.contains("1\n00:00:01,000 --> 00:00:02,000\n"));
    assert!(
        text.chars()
            .any(|character| ('А'..='я').contains(&character))
    );
    Ok(())
}
