use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationLinkStore, connect_sqlite,
    },
};
use adapters_translate::{FormatInspector, TranslateResultReader, TranslateRunRegistrar};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, InspectTranslationSourceUseCase,
    PublishTranslationRequest, PublishTranslationResultUseCase,
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use auralis_translation::{
    BlockPolicy, LanguageCode, LanguagePair, ProviderError, ProviderResponse, ResultId,
    ReviewState, RunId, TargetSegment, TranslationBatch, TranslationProvider,
};
use auralis_translation_formats::srt::SrtRunPlan;
use auralis_translation_sqlite::{ResultSpec, SqliteConfig, TranslateDb};
use domain::media::{ArtifactId, ArtifactKind, ArtifactState};
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationState};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);

struct FakeProvider;

impl TranslationProvider for FakeProvider {
    fn translate(&self, batch: &TranslationBatch) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse {
            schema_version: 1,
            translations: batch
                .targets()
                .iter()
                .map(|segment| TargetSegment {
                    id: segment.id(),
                    lines: vec!["Привет.".into()],
                })
                .collect(),
        })
    }
}

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

    let mut translate = TranslateDb::open(&translate_path, SqliteConfig::default())?;
    let run_id_typed = RunId::new(run_id).ok_or("nil run ID")?;
    let run = translate.run(run_id_typed)?;
    let plan = SrtRunPlan::new(
        source,
        run.translation_id,
        run_id_typed,
        LanguagePair::new(LanguageCode::Chinese, LanguageCode::Russian)?,
        BlockPolicy::default(),
    )?;
    translate.begin_initial_attempt(&run, None)?;
    let expected_output = plan.execute(&FakeProvider, &mut translate)?;
    let result_id = ResultId::new(Uuid::new_v4()).ok_or("nil result ID")?;
    translate.commit_result(
        &ResultSpec {
            result_id,
            run_id: run_id_typed,
            revision: 1,
            source_hash: plan.source_hash(),
            block_fingerprints: plan.block_fingerprints(),
            review_state: ReviewState::NeedsReview,
        },
        &plan,
    )?;
    drop(translate);

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
        result_id: result_id.get(),
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
    assert_eq!(selected.selected_result_id, Some(result_id.get()));
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
    assert!(String::from_utf8(expected_output)?.contains("Привет."));
    Ok(())
}
