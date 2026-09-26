use super::historical_seed;
use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationEditIntentStore, SqliteTranslationLinkStore,
        connect_sqlite,
    },
};
use adapters_translate::{
    FormatInspector, TranslateResultEditor, TranslateResultReader, TranslateRunControl,
    TranslateRunRegistrar,
};
use application::{
    usecases::translation::*,
    worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig},
};
use domain::{
    media::ArtifactKind,
    project::{Project, ProjectId},
};
use ports::{
    artifact_index::ArtifactIndex,
    repository::ProjectRepository,
    storage::ArtifactStore,
    translation_inspection::TranslationSourceFormat,
    translation_links::{StartTranslationRun, TranslationLinkStore},
    translation_run_registration::TranslationRunRegistrationPort,
};
use std::{error::Error, path::PathBuf, sync::Arc};
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);

struct Events;
#[async_trait::async_trait]
impl ports::events::AppEventPublisher for Events {
    async fn publish_project_updated(&self, _: &str) -> Result<(), ports::PortError> {
        Ok(())
    }
    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), ports::PortError> {
        Ok(())
    }
}

pub struct HistoricalFixture {
    pub directory: tempfile::TempDir,
    pub pool: sqlx::SqlitePool,
    pub links: Arc<SqliteTranslationLinkStore>,
    pub intents: Arc<SqliteTranslationEditIntentStore>,
    pub artifacts: Arc<SqliteArtifactIndex>,
    pub files: Arc<LocalArtifactStore>,
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub run_id: Uuid,
    pub base_id: Uuid,
    pub head_id: Uuid,
    pub source: Vec<u8>,
    pub source_path: PathBuf,
    pub translate_path: PathBuf,
    pub format: TranslationSourceFormat,
}

impl HistoricalFixture {
    pub async fn new(format: TranslationSourceFormat) -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
        let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
        let intents = Arc::new(SqliteTranslationEditIntentStore::new(pool.clone()));
        let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
        let files = Arc::new(LocalArtifactStore::new(directory.path().join("files")));
        let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
        let project_id = projects
            .create(Project::new("Historical branches".into())?)
            .await?
            .id()
            .clone();
        let source = match format {
            TranslationSourceFormat::Srt => "1\r\n00:00:01,000 --> 00:00:02,000\r\n你好。\r\n\r\n2\r\n00:00:02,000 --> 00:00:03,000\r\n再见。\r\n",
            TranslationSourceFormat::Vtt => "WEBVTT\r\n\r\nNOTE provenance\r\nfixture\r\n\r\nfirst\r\n00:01.000 --> 00:02.000\r\n你好。\r\n\r\nsecond\r\n00:02.000 --> 00:03.000\r\n再见。\r\n",
        }.as_bytes().to_vec();
        let external = directory
            .path()
            .join(if format == TranslationSourceFormat::Srt {
                "input.srt"
            } else {
                "input.vtt"
            });
        std::fs::write(&external, &source)?;
        let source_artifact = ImportTranslationSourceUseCase::new(
            projects.clone(),
            files.clone(),
            Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
            Arc::new(FormatInspector),
        )
        .execute(ImportTranslationSourceRequest {
            project_id: project_id.clone(),
            path: external,
            format,
        })
        .await?;
        let translation_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let translate_path = directory.path().join("translate.sqlite");
        let mut fixture = Self {
            directory,
            pool,
            links,
            intents,
            artifacts,
            files,
            project_id,
            translation_id,
            run_id,
            base_id: Uuid::nil(),
            head_id: Uuid::nil(),
            source,
            source_path: PathBuf::new(),
            translate_path,
            format,
        };
        fixture.flush().await?;
        let original = fixture
            .artifacts
            .list_by_project_and_kind(&fixture.project_id, ArtifactKind::OriginalSubtitle)
            .await?
            .into_iter()
            .find(|artifact| artifact.id == source_artifact.id)
            .ok_or("missing source")?;
        fixture.source_path = fixture.files.resolve_artifact(&original).await?;
        BeginTranslationUseCase::new(
            fixture.inspector(),
            fixture.links.clone(),
            Arc::new(TranslateRunRegistrar::new(fixture.translate_path.clone())),
        )
        .execute(BeginTranslationRequest {
            project_id: fixture.project_id.clone(),
            source_artifact_id: source_artifact.id,
            format,
            source_language: "zh".into(),
            translation_id,
            run_id,
            profile_manifest_json: PROFILE.into(),
            created_at: domain::chrono::Utc::now(),
        })
        .await?;
        let intent = fixture
            .links
            .run_intent(run_id)
            .await?
            .ok_or("missing run")?;
        fixture.base_id = historical_seed::seed(
            &fixture.translate_path,
            &fixture.source,
            if format == TranslationSourceFormat::Srt {
                "srt"
            } else {
                "vtt"
            },
            &intent,
        )?;
        fixture.publish(fixture.base_id).await?;
        fixture.flush().await?;
        fixture.head_id = Uuid::new_v4();
        EditTranslationResultUseCase::new(
            fixture.inspector(),
            fixture.links.clone(),
            Arc::new(TranslateResultEditor::new(fixture.translate_path.clone())),
        )
        .execute(EditTranslationResultRequest {
            project_id: fixture.project_id.clone(),
            translation_id,
            base_result_id: fixture.base_id,
            result_id: fixture.head_id,
            segment_id: 2,
            lines: vec!["Поздняя правка второго.".into()],
        })
        .await?;
        fixture.publish(fixture.head_id).await?;
        fixture.flush().await?;
        Ok(fixture)
    }

    pub fn inspector(&self) -> Arc<InspectTranslationSourceUseCase> {
        Arc::new(InspectTranslationSourceUseCase::new(
            Arc::new(SqliteProjectRepository::new(self.pool.clone())),
            self.artifacts.clone(),
            self.files.clone(),
            Arc::new(FormatInspector),
        ))
    }
    pub fn verifier(&self) -> Arc<VerifyTranslationHistoryUseCase> {
        Arc::new(VerifyTranslationHistoryUseCase::new(
            self.inspector(),
            self.links.clone(),
            Arc::new(TranslateResultReader::new(self.translate_path.clone())),
            self.artifacts.clone(),
            self.files.clone(),
        ))
    }
    pub fn editor(&self) -> EditHistoricalTranslationUseCase {
        EditHistoricalTranslationUseCase::new(
            self.verifier(),
            self.intents.clone(),
            Arc::new(TranslateResultEditor::new(self.translate_path.clone())),
        )
    }
    pub fn publisher(&self) -> PublishTranslationResultUseCase {
        PublishTranslationResultUseCase::new(
            self.inspector(),
            self.links.clone(),
            Arc::new(TranslateResultReader::new(self.translate_path.clone())),
            self.files.clone(),
        )
        .with_edit_intents(self.intents.clone())
    }
    pub fn recovery(&self) -> RecoverTranslationPublicationsUseCase {
        RecoverTranslationPublicationsUseCase::new(
            self.links.clone(),
            Arc::new(TranslateRunControl::new(self.translate_path.clone())),
            self.publisher(),
        )
        .with_edit_intents(self.intents.clone())
    }
    pub async fn publish(&self, result_id: Uuid) -> Result<(), Box<dyn Error>> {
        self.publisher()
            .execute(PublishTranslationRequest {
                project_id: self.project_id.clone(),
                translation_id: self.translation_id,
                result_id,
            })
            .await?;
        Ok(())
    }
    pub async fn flush(&self) -> Result<(), Box<dyn Error>> {
        OutboxWorker::new(
            SqliteOutboxRepository::new(self.pool.clone()),
            self.files.clone(),
            self.artifacts.clone(),
            Arc::new(SqliteStorageUnitOfWork::new(self.pool.clone())),
            Arc::new(Events),
            Arc::new(LocalTempWorkspace::new(self.directory.path())),
            OutboxMaintenanceConfig::try_default()?,
        )
        .with_translations(self.links.clone())
        .process_pending_messages(100)
        .await?;
        Ok(())
    }
    pub async fn request(&self) -> Result<EditHistoricalTranslationRequest, Box<dyn Error>> {
        Ok(EditHistoricalTranslationRequest {
            project_id: self.project_id.clone(),
            translation_id: self.translation_id,
            base_result_id: self.base_id,
            result_id: Uuid::new_v4(),
            expected_head_result_id: self.head_id,
            expected_link_revision: self
                .links
                .get(self.translation_id)
                .await?
                .ok_or("missing link")?
                .revision,
            segment_id: 1,
            lines: vec!["Историческая правка первого.".into()],
        })
    }
    pub async fn reopen(&mut self) -> Result<(), Box<dyn Error>> {
        self.pool.close().await;
        self.pool = connect_sqlite(self.directory.path().join("auralis.sqlite")).await?;
        self.links = Arc::new(SqliteTranslationLinkStore::new(self.pool.clone()));
        self.intents = Arc::new(SqliteTranslationEditIntentStore::new(self.pool.clone()));
        self.artifacts = Arc::new(SqliteArtifactIndex::new(self.pool.clone()));
        Ok(())
    }
    pub async fn start_other_run(&self) -> Result<Uuid, Box<dyn Error>> {
        let link = self
            .links
            .get(self.translation_id)
            .await?
            .ok_or("missing link")?;
        let registrar = TranslateRunRegistrar::new(self.translate_path.clone());
        let run_id = Uuid::new_v4();
        let intent = registrar.prepare_intent(
            self.format,
            self.translation_id,
            run_id,
            PROFILE.into(),
            domain::chrono::Utc::now(),
        )?;
        self.links
            .start_run(StartTranslationRun {
                translation_id: self.translation_id,
                run_id,
                expected_active_run_id: link.active_run_id,
                expected_link_revision: link.revision,
                updated_at: domain::chrono::Utc::now(),
            })
            .await?;
        let started = self
            .links
            .get(self.translation_id)
            .await?
            .ok_or("missing started link")?;
        self.links
            .begin_with_run_intent(
                ports::translation_links::TranslationIntent {
                    translation_id: self.translation_id,
                    project_id: self.project_id.clone(),
                    source_artifact_id: link.source_artifact_id,
                    source_sha256: link.source_sha256,
                    source_format: link.source_format,
                    source_language: link.source_language,
                    target_language: link.target_language,
                    run_id,
                    created_at: intent.created_at,
                },
                intent.clone(),
            )
            .await?;
        registrar
            .ensure_run(&self.source_path, &started, &intent)
            .await?;
        Ok(run_id)
    }
}
