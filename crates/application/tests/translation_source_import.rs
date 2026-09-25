use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationLinkStore, connect_sqlite,
    },
};
use adapters_translate::{FormatInspector, TranslateRunRegistrar};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ImportTranslationSourceRequest,
    ImportTranslationSourceUseCase, InspectTranslationSourceRequest,
    InspectTranslationSourceUseCase,
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::media::{ArtifactKind, ArtifactState};
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::TranslationLinkStore;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
);

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
async fn imported_subtitle_is_a_verified_managed_copy_and_invalid_input_is_rejected()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let store = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
    let project = projects.create(Project::new("Translation".into())?).await?;
    let original_path = directory.path().join("incoming.srt");
    let original = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
    std::fs::write(&original_path, original)?;

    let importer = ImportTranslationSourceUseCase::new(
        projects.clone(),
        store.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(FormatInspector),
    );
    let pending = importer
        .execute(ImportTranslationSourceRequest {
            project_id: project.id().clone(),
            path: original_path.clone(),
            format: TranslationSourceFormat::Srt,
        })
        .await?;
    assert_eq!(pending.kind, ArtifactKind::OriginalSubtitle);
    assert_eq!(pending.state, ArtifactState::PendingFinalize);
    assert_eq!(std::fs::read(&original_path)?, original);

    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        store.clone(),
        artifacts.clone(),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(directory.path())),
        OutboxMaintenanceConfig::try_default()?,
    );
    assert_eq!(worker.process_pending_messages(10).await?.fetched, 1);
    let ready = artifacts
        .list_by_project_and_kind(project.id(), ArtifactKind::OriginalSubtitle)
        .await?
        .into_iter()
        .find(|artifact| artifact.id == pending.id)
        .ok_or("missing imported original")?;
    assert_eq!(ready.state, ArtifactState::Ready);
    assert_eq!(
        std::fs::read(store.resolve_artifact(&ready).await?)?,
        original
    );

    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects.clone(),
        artifacts.clone(),
        store,
        Arc::new(FormatInspector),
    ));
    let report = inspector
        .execute(InspectTranslationSourceRequest {
            project_id: project.id().clone(),
            source_artifact_id: ready.id.clone(),
            format: TranslationSourceFormat::Srt,
        })
        .await?;
    assert_eq!(report.segments[0].source_lines, ["你好。"]);
    let links = Arc::new(SqliteTranslationLinkStore::new(pool));
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let linked = BeginTranslationUseCase::new(
        inspector,
        links.clone(),
        Arc::new(TranslateRunRegistrar::new(
            directory.path().join("translate.sqlite"),
        )),
    )
    .execute(BeginTranslationRequest {
        project_id: project.id().clone(),
        source_artifact_id: ready.id,
        format: TranslationSourceFormat::Srt,
        source_language: "zh".into(),
        translation_id,
        run_id,
        profile_manifest_json: PROFILE.into(),
        created_at: domain::chrono::Utc::now(),
    })
    .await?;
    assert_eq!(linked.active_run_id, Some(run_id));
    assert_eq!(
        links
            .get(translation_id)
            .await?
            .ok_or("missing link")?
            .source_sha256,
        report.source_sha256
    );
    assert_eq!(std::fs::read(&original_path)?, original);

    let invalid_path = directory.path().join("invalid.srt");
    std::fs::write(&invalid_path, b"not a subtitle")?;
    assert!(
        importer
            .execute(ImportTranslationSourceRequest {
                project_id: project.id().clone(),
                path: invalid_path.clone(),
                format: TranslationSourceFormat::Srt,
            })
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&invalid_path)?, b"not a subtitle");
    let wrong_extension = directory.path().join("valid.txt");
    std::fs::write(&wrong_extension, original)?;
    assert!(
        importer
            .execute(ImportTranslationSourceRequest {
                project_id: project.id().clone(),
                path: wrong_extension,
                format: TranslationSourceFormat::Srt,
            })
            .await
            .is_err()
    );
    assert_eq!(
        artifacts
            .list_by_project_and_kind(project.id(), ArtifactKind::OriginalSubtitle)
            .await?
            .len(),
        1
    );
    Ok(())
}
