use adapters_storage::{
    local::LocalArtifactStore,
    sqlite::{
        SqliteArtifactIndex, SqliteJobRepository, SqliteProjectRepository, SqliteStorageUnitOfWork,
        SqliteTranslationHostJobStore, SqliteTranslationLinkStore, connect_sqlite,
    },
};
use adapters_translate::{
    FormatInspector, TranslateResultReader, TranslateRunControl, TranslateRunExecutor,
    TranslateRunRegistrar,
};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, ExecuteTranslationUseCase,
    InspectTranslationSourceUseCase, PublishTranslationResultUseCase, RunManagedTranslationUseCase,
    RunTranslationHostJobUseCase, ScheduleManagedTranslationUseCase, StartTranslationJobUseCase,
    TranslationRunRequest, TranslationRunUseCase,
};
use async_trait::async_trait;
use domain::job::JobStatus;
use domain::media::ArtifactId;
use domain::project::Project;
use jobs::manager::JobManager;
use ports::error::PortError;
use ports::job_runtime_control::JobRuntimeControlPort;
use ports::repository::{JobRepository, ProjectRepository};
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLinkStore, TranslationRunIntent};
use ports::translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort};
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);
const SOURCE: &[u8] = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";

struct UnavailableRuntime;

struct FixedLease;

impl TranslationModelLease for FixedLease {
    fn server_url(&self) -> &str {
        "http://127.0.0.1:1/"
    }
}

struct FixedRuntime;

#[async_trait]
impl TranslationModelRuntimePort for FixedRuntime {
    async fn acquire(
        &self,
        _: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        Ok(Box::new(FixedLease))
    }
}

#[async_trait]
impl TranslationModelRuntimePort for UnavailableRuntime {
    async fn acquire(
        &self,
        _: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        Err(PortError::Busy {
            message: "model slot unavailable".into(),
        })
    }
}

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
    let control = Arc::new(TranslateRunControl::new(translate_path.clone()));
    let jobs = Arc::new(SqliteTranslationHostJobStore::new(pool.clone()));
    let request = TranslationRunRequest {
        project_id: project.id().clone(),
        translation_id,
    };
    let executor = Arc::new(ExecuteTranslationUseCase::new(
        inspector.clone(),
        links.clone(),
        registrar.clone(),
        Arc::new(TranslateRunExecutor::experimental_unchecked(
            translate_path.clone(),
        )),
    ));
    let managed = RunManagedTranslationUseCase::new(
        links.clone(),
        Arc::new(UnavailableRuntime),
        control.clone(),
        StartTranslationJobUseCase::new(links.clone(), control.clone(), jobs.clone()),
        RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), executor.clone()),
        PublishTranslationResultUseCase::new(
            inspector.clone(),
            links.clone(),
            Arc::new(TranslateResultReader::new(translate_path.clone())),
            files.clone(),
        ),
    );
    assert!(managed.execute(request.clone()).await.is_err());
    assert!(jobs.active_for_run(run_id).await?.is_none());
    let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(job_count, 0);
    let managed = RunManagedTranslationUseCase::new(
        links.clone(),
        Arc::new(FixedRuntime),
        control.clone(),
        StartTranslationJobUseCase::new(links.clone(), control.clone(), jobs.clone()),
        RunTranslationHostJobUseCase::new(links.clone(), jobs.clone(), executor),
        PublishTranslationResultUseCase::new(
            inspector,
            links.clone(),
            Arc::new(TranslateResultReader::new(translate_path)),
            files,
        ),
    );
    let attempt = managed.admit(request).await?;
    let job = attempt.job().clone();
    assert_eq!(attempt.run_id(), run_id);
    assert_eq!(job.status(), &JobStatus::Pending);
    assert_eq!(
        jobs.active_for_run(run_id)
            .await?
            .as_ref()
            .map(|item| item.id()),
        Some(job.id())
    );
    let link = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(
        control.snapshot(&link).await?.phase,
        TranslationRunPhase::Requested
    );
    let failed = attempt.execute().await;
    assert!(failed.is_err());
    assert!(jobs.active_for_run(run_id).await?.is_none());
    let failed_job = SqliteJobRepository::new(pool.clone())
        .get(job.id())
        .await?
        .ok_or("missing failed job")?;
    assert_eq!(failed_job.status(), &JobStatus::Failed);
    assert_eq!(failed_job.progress().processed_items, Some(0));
    let link = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(
        control.snapshot(&link).await?.phase,
        TranslationRunPhase::Failed
    );
    let retry = StartTranslationJobUseCase::new(links.clone(), control.clone(), jobs.clone())
        .execute(TranslationRunRequest {
            project_id: project.id().clone(),
            translation_id,
        })
        .await?;
    assert_ne!(retry.id(), job.id());
    let manager = Arc::new(JobManager::new(
        Arc::new(SqliteJobRepository::new(pool.clone())),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        None,
    ));
    let scheduler = ScheduleManagedTranslationUseCase::new(
        managed,
        manager.clone(),
        jobs.clone(),
        TranslationRunUseCase::new(links, control),
    );
    manager
        .reserve(retry.id().clone(), project.id().clone())
        .await?;
    let (duplicate, duplicate_run_id) = scheduler
        .execute(TranslationRunRequest {
            project_id: project.id().clone(),
            translation_id,
        })
        .await?;
    assert_eq!(duplicate.id(), retry.id());
    assert_eq!(duplicate_run_id, run_id);
    assert_eq!(jobs.active_for_run(run_id).await?, Some(retry.clone()));
    assert_eq!(
        SqliteJobRepository::new(pool.clone())
            .get(retry.id())
            .await?
            .ok_or("missing reserved job")?
            .status(),
        &JobStatus::Pending
    );
    manager.rollback_runtime_start(retry.id()).await?;
    let (scheduled, scheduled_run_id) = scheduler
        .execute(TranslationRunRequest {
            project_id: project.id().clone(),
            translation_id,
        })
        .await?;
    assert_eq!(scheduled.id(), retry.id());
    assert_eq!(scheduled_run_id, run_id);
    let deadline = Instant::now() + Duration::from_secs(10);
    while jobs.active_for_run(run_id).await?.is_some() {
        if Instant::now() >= deadline {
            return Err("background translation host job did not finish".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let scheduled_failure = SqliteJobRepository::new(pool)
        .get(retry.id())
        .await?
        .ok_or("missing scheduled job")?;
    assert_eq!(scheduled_failure.status(), &JobStatus::Failed);
    let shutdown = manager.drain_all(Duration::from_secs(1)).await?;
    assert_eq!(shutdown.unconfirmed_count, 0);
    assert_eq!(std::fs::read(source_path)?, SOURCE);
    Ok(())
}
