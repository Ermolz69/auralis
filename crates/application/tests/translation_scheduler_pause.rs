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
use domain::job::JobStatus;
use domain::media::ArtifactId;
use domain::project::Project;
use jobs::manager::JobManager;
use ports::job_runtime_control::JobRuntimeControlPort;
use ports::repository::{JobRepository, ProjectRepository};
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::TranslationLinkStore;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);
const SOURCE: &[u8] = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";

#[path = "support/loopback_translation_runtime.rs"]
mod loopback_translation_runtime;
#[path = "support/observed_job_runtime.rs"]
mod observed_job_runtime;
use loopback_translation_runtime::LoopbackTranslationRuntime;
use observed_job_runtime::ObservedJobRuntime;

#[tokio::test]
async fn durable_pause_finishes_scheduler_as_cancelled_without_runtime_token_cancellation()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let project = projects.create(Project::new("Pause".into())?).await?;
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
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let endpoint = format!("http://{}/", listener.local_addr()?);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let server = std::thread::spawn(move || -> Result<(), String> {
        use std::io::Read;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return Err("no model request".into());
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        stream
            .set_nonblocking(false)
            .map_err(|error| error.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| error.to_string())?;
        let mut buffer = [0; 4096];
        if stream
            .read(&mut buffer)
            .map_err(|error| error.to_string())?
            == 0
        {
            return Err("empty model request".into());
        }
        ready_tx.send(()).map_err(|_| "request waiter closed")?;
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => return Ok(()),
                Err(error) => return Err(error.to_string()),
            }
        }
    });
    let managed = RunManagedTranslationUseCase::new(
        links.clone(),
        Arc::new(LoopbackTranslationRuntime(endpoint)),
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
    let manager = Arc::new(JobManager::new(
        Arc::new(SqliteJobRepository::new(pool.clone())),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        None,
    ));
    let observed = Arc::new(ObservedJobRuntime::new(manager.clone()));
    let pause = TranslationRunUseCase::new(links.clone(), control.clone());
    let scheduler = ScheduleManagedTranslationUseCase::new(
        managed,
        observed.clone(),
        jobs.clone(),
        pause.clone(),
    );
    let (job, scheduled_run_id) = scheduler.execute(request.clone()).await?;
    assert_eq!(scheduled_run_id, run_id);
    let completion = observed.completion().ok_or("missing attached task")?;
    tokio::time::timeout(Duration::from_secs(5), ready_rx).await??;
    pause.pause(request).await?;
    let outcome = tokio::time::timeout(Duration::from_secs(3), completion.wait()).await?;
    assert_eq!(
        outcome,
        ports::job_runtime_control::RuntimeTaskOutcome::Cancelled
    );
    server.join().map_err(|_| "model server panicked")??;
    assert!(jobs.active_for_run(run_id).await?.is_none());
    let cancelled = SqliteJobRepository::new(pool)
        .get(job.id())
        .await?
        .ok_or("missing cancelled job")?;
    assert_eq!(cancelled.status(), &JobStatus::Cancelled);
    let link = links.get(translation_id).await?.ok_or("missing link")?;
    let snapshot = control.snapshot(&link).await?;
    assert_eq!(snapshot.phase, TranslationRunPhase::Paused);
    assert!(!snapshot.pause_requested);
    assert_eq!(link.active_run_id, Some(run_id));
    assert!(link.selected_result_id.is_none());
    assert_eq!(std::fs::read(source_path)?, SOURCE);
    let shutdown = manager.drain_all(Duration::from_secs(1)).await?;
    assert_eq!(shutdown.unconfirmed_count, 0);
    Ok(())
}
