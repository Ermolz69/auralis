use adapters_storage::{
    local::LocalArtifactStore,
    sqlite::{
        SqliteArtifactIndex, SqliteProjectRepository, SqliteTranslationHostJobStore,
        SqliteTranslationLinkStore, connect_sqlite,
    },
};
use adapters_translate::{
    FormatInspector, TranslateResultReader, TranslateRunControl, TranslateRunExecutor,
    TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ExecuteTranslationUseCase,
    InspectTranslationSourceUseCase, PublishTranslationResultUseCase, RunManagedTranslationUseCase,
    RunTranslationHostJobUseCase, StartTranslationJobUseCase, TranslationRunRequest,
    TranslationRunUseCase,
};
use domain::{media::ArtifactId, project::Project};
use ports::{
    repository::ProjectRepository, translation_inspection::TranslationSourceFormat,
    translation_model_runtime::TranslationModelRuntimePort,
    translation_run_control::TranslationRunControlPort,
};
use std::{error::Error, path::PathBuf, sync::Arc};
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);
pub const SOURCE: &[u8] =
    b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";

pub struct TranslationAdmissionFixture {
    pub managed: RunManagedTranslationUseCase,
    pub pause: TranslationRunUseCase,
    pub request: TranslationRunRequest,
    pub run_id: Uuid,
    pub pool: sqlx::SqlitePool,
    pub translate_path: PathBuf,
    pub source_path: PathBuf,
    pub links: Arc<SqliteTranslationLinkStore>,
    pub jobs: Arc<SqliteTranslationHostJobStore>,
    _directory: tempfile::TempDir,
}

impl TranslationAdmissionFixture {
    pub async fn new(
        runtime: Arc<dyn TranslationModelRuntimePort>,
    ) -> Result<Self, Box<dyn Error>> {
        Self::new_with_control(runtime, |control| control).await
    }

    pub async fn new_with_control(
        runtime: Arc<dyn TranslationModelRuntimePort>,
        wrap_control: impl FnOnce(
            Arc<dyn TranslationRunControlPort>,
        ) -> Arc<dyn TranslationRunControlPort>,
    ) -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
        let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
        let project = projects
            .create(Project::new("Admission race".into())?)
            .await?;
        let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
        let files = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
        let source_id = ArtifactId::new();
        let key = format!("projects/{}/original.srt", project.id());
        let source_path = files.resolve_storage_key(&key)?;
        std::fs::create_dir_all(source_path.parent().ok_or("missing source parent")?)?;
        std::fs::write(&source_path, SOURCE)?;
        let now = domain::chrono::Utc::now();
        sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, ?, 'ready', ?, ?)")
            .bind(source_id.to_string()).bind(project.id().to_string()).bind(&key)
            .bind(i64::try_from(SOURCE.len())?).bind(now.to_rfc3339()).bind(now.to_rfc3339())
            .execute(&pool).await?;
        let inspector = Arc::new(InspectTranslationSourceUseCase::new(
            projects,
            artifacts,
            files.clone(),
            Arc::new(FormatInspector),
        ));
        let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
        let translate_path = directory.path().join("translate.sqlite");
        let translation_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let registrar = Arc::new(TranslateRunRegistrar::new(translate_path.clone()));
        BeginTranslationUseCase::new(inspector.clone(), links.clone(), registrar.clone())
            .execute(BeginTranslationRequest {
                project_id: project.id().clone(),
                source_artifact_id: source_id,
                format: TranslationSourceFormat::Srt,
                source_language: "zh".into(),
                translation_id,
                run_id,
                profile_manifest_json: PROFILE.into(),
                created_at: now,
            })
            .await?;
        let control = wrap_control(Arc::new(TranslateRunControl::new(translate_path.clone())));
        let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
        let request = TranslationRunRequest {
            project_id: project.id().clone(),
            translation_id,
        };
        let executor = Arc::new(ExecuteTranslationUseCase::new(
            inspector.clone(),
            links.clone(),
            registrar,
            Arc::new(TranslateRunExecutor::experimental_unchecked(
                translate_path.clone(),
            )),
        ));
        let managed = RunManagedTranslationUseCase::new(
            links.clone(),
            runtime,
            control.clone(),
            StartTranslationJobUseCase::new(links.clone(), control.clone(), jobs.clone()),
            RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), executor),
            PublishTranslationResultUseCase::new(
                inspector,
                links.clone(),
                Arc::new(TranslateResultReader::new(translate_path.clone())),
                files,
            ),
        );
        Ok(Self {
            managed,
            pause: TranslationRunUseCase::new(links.clone(), control),
            request,
            run_id,
            pool,
            translate_path,
            source_path,
            links,
            jobs,
            _directory: directory,
        })
    }
}
