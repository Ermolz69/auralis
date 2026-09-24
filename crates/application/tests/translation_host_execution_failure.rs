use adapters_storage::{
    local::LocalArtifactStore,
    sqlite::{
        SqliteArtifactIndex, SqliteJobRepository, SqliteProjectRepository,
        SqliteTranslationHostJobStore, SqliteTranslationLinkStore, connect_sqlite,
    },
};
use adapters_translate::{
    FormatInspector, TranslateRunControl, TranslateRunExecutor, TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ExecuteTranslationRequest,
    ExecuteTranslationUseCase, InspectTranslationSourceUseCase, RunTranslationHostJobUseCase,
    StartTranslationJobUseCase, TranslationRunRequest,
};
use domain::job::JobStatus;
use domain::media::ArtifactId;
use domain::project::Project;
use ports::repository::{JobRepository, ProjectRepository};
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::TranslationLinkStore;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);
const SOURCE: &[u8] = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";

#[tokio::test]
async fn failed_model_request_closes_host_job_without_changing_source() -> Result<(), Box<dyn Error>>
{
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let project = projects.create(Project::new("Failure".into())?).await?;
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let files = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
    let source_id = ArtifactId::new();
    let key = format!("projects/{}/original.srt", project.id());
    let source_path = files.resolve_storage_key(&key)?;
    std::fs::create_dir_all(source_path.parent().ok_or("missing source parent")?)?;
    std::fs::write(&source_path, SOURCE)?;
    let now = domain::chrono::Utc::now();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, ?, 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&key)
        .bind(i64::try_from(SOURCE.len())?)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&pool)
        .await?;
    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects,
        artifacts,
        files,
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
    let control = Arc::new(TranslateRunControl::new(translate_path.clone()));
    let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
    let request = TranslationRunRequest {
        project_id: project.id().clone(),
        translation_id,
    };
    let job = StartTranslationJobUseCase::new(links.clone(), control.clone(), jobs.clone())
        .execute(request)
        .await?;
    let executor = Arc::new(ExecuteTranslationUseCase::new(
        inspector,
        links.clone(),
        registrar,
        Arc::new(TranslateRunExecutor::experimental_unchecked(translate_path)),
    ));
    let failed = RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), executor)
        .execute(
            ExecuteTranslationRequest {
                project_id: project.id().clone(),
                translation_id,
                server_url: "http://127.0.0.1:1/".into(),
            },
            job.id(),
        )
        .await;
    assert!(failed.is_err());
    assert!(jobs.active_for_run(run_id).await?.is_none());
    assert_eq!(
        SqliteJobRepository::new(pool)
            .get(job.id())
            .await?
            .ok_or("missing failed job")?
            .status(),
        &JobStatus::Failed
    );
    let link = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(
        control.snapshot(&link).await?.phase,
        TranslationRunPhase::Failed
    );
    let retry = StartTranslationJobUseCase::new(links, control, jobs)
        .execute(TranslationRunRequest {
            project_id: project.id().clone(),
            translation_id,
        })
        .await?;
    assert_ne!(retry.id(), job.id());
    assert_eq!(std::fs::read(source_path)?, SOURCE);
    Ok(())
}
