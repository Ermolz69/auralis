#![allow(dead_code, clippy::unwrap_used)]

use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteJobRepository, SqliteProjectRepository, SqliteStorageUnitOfWork, connect_sqlite,
        job_executions::SqliteJobExecutionStore,
    },
};
use domain::{
    job::{Job, JobId, JobKind, JobStatus},
    project::Project,
};
use jobs::{manager::JobManager, queue::JobQueueConfig};
use ports::{
    PortError,
    job_execution::{
        ExecutionContext, ExecutionDescriptor, JobExecutorPort, JobQueuePort, QueuedExecution,
    },
    job_resources::ResourceDemand,
    job_runtime_control::{JobRuntimeControlPort, RuntimeTaskOutcome},
    job_scheduler::{JobLifecycleEvent, JobSchedulerPort},
    repository::{JobRepository, ProjectRepository},
    transaction::CommitPipelineStart,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc, oneshot};

pub const CPU: ResourceDemand = ResourceDemand {
    cpu: 1,
    network: 0,
    disk: 0,
    processes: 0,
};
pub const NETWORK: ResourceDemand = ResourceDemand {
    cpu: 0,
    network: 1,
    disk: 0,
    processes: 0,
};

pub struct Harness {
    pub root: tempfile::TempDir,
    pub pool: sqlx::SqlitePool,
    pub projects: Arc<SqliteProjectRepository>,
    pub jobs: Arc<SqliteJobRepository>,
    pub store: Arc<SqliteJobExecutionStore>,
    pub uow: Arc<SqliteStorageUnitOfWork>,
    pub manager: Arc<JobManager>,
    pub events: broadcast::Sender<JobLifecycleEvent>,
}

impl Harness {
    pub async fn new(config: JobQueueConfig) -> Self {
        let root = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(root.path().join("auralis.sqlite"))
            .await
            .unwrap();
        let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
        let jobs = Arc::new(SqliteJobRepository::new(pool.clone()));
        let uow = Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));
        let store = Arc::new(SqliteJobExecutionStore::new(pool.clone()));
        let (events, _) = broadcast::channel::<JobLifecycleEvent>(256);
        let emitter = events.clone();
        let manager = Arc::new(
            JobManager::new(
                jobs.clone(),
                uow.clone(),
                Some(Arc::new(move |event| {
                    let _ = emitter.send(event);
                })),
            )
            .with_queue(store.clone(), config)
            .unwrap(),
        );
        Self {
            root,
            pool,
            projects,
            jobs,
            uow,
            store,
            manager,
            events,
        }
    }

    pub async fn controlled(&self) -> mpsc::UnboundedReceiver<Started> {
        let (send, receive) = mpsc::unbounded_channel();
        self.manager
            .start_dispatcher(Arc::new(ControlledExecutor {
                manager: self.manager.clone(),
                started: send,
            }))
            .await
            .unwrap();
        receive
    }

    pub async fn project(&self) -> Project {
        let mut project = Project::new("Local queue fixture".into()).unwrap();
        project
            .import_source(
                domain::media::MediaSource::YoutubeUrl {
                    url: "https://youtube.com/watch?v=queue-fixture".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        self.projects.create(project).await.unwrap()
    }

    pub async fn admit(&self, demand: ResourceDemand) -> JobId {
        let mut project = self.project().await;
        let job = Job::new(
            project.id().clone(),
            "Queue fixture".into(),
            JobKind::Dubbing,
        );
        project.start_processing(job.id().clone()).unwrap();
        let mut descriptor = ExecutionDescriptor::subtitle_import(project.id().clone(), None);
        descriptor.resources = demand;
        self.manager
            .admit_pipeline(CommitPipelineStart { project, job }, descriptor)
            .await
            .unwrap()
            .id
    }

    pub async fn status(&self, id: &JobId) -> JobStatus {
        self.jobs.get(id).await.unwrap().unwrap().status().clone()
    }

    pub async fn wait_status(&self, id: &JobId, status: JobStatus) {
        let mut events = self.events.subscribe();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if self.status(id).await == status {
                    return;
                }
                let event = events.recv().await.unwrap();
                if &event.job.id == id && event.job.status == status {
                    return;
                }
            }
        })
        .await
        .unwrap();
    }

    pub async fn shutdown(&self) {
        self.manager
            .drain_all(Duration::from_secs(2))
            .await
            .unwrap();
    }

    pub fn pipeline<V: ports::source::SubtitleSourcePort + Clone + 'static>(
        &self,
        source: V,
        workspace: Arc<dyn ports::workspace::TempWorkspacePort>,
    ) -> application::usecases::pipeline::start_mock::StartMockPipelineUseCase<
        Arc<SqliteProjectRepository>,
        V,
        Arc<LocalArtifactStore>,
    > {
        use application::usecases::pipeline::start_mock::{
            StartMockPipelineDependencies, StartMockPipelineUseCase,
        };
        StartMockPipelineUseCase::new(StartMockPipelineDependencies {
            project_repo: self.projects.clone(),
            job_scheduler: self.manager.clone(),
            storage_uow: self.uow.clone(),
            subtitle_source: source,
            artifact_store: Arc::new(LocalArtifactStore::new(self.root.path().join("projects"))),
            workspace_port: workspace,
            locks: Arc::new(
                application::usecases::project::lifecycle::ProjectLifecycleLocks::new(),
            ),
            job_runtime: self.manager.clone(),
        })
        .with_queue(self.manager.clone())
    }

    pub fn workspace(&self) -> Arc<LocalTempWorkspace> {
        let root = self.root.path().join("workspaces");
        std::fs::create_dir_all(&root).unwrap();
        Arc::new(LocalTempWorkspace::new(root))
    }
}

pub struct Started {
    pub id: JobId,
    pub finish: oneshot::Sender<bool>,
    pub cleanup: oneshot::Sender<()>,
}

pub async fn started(events: &mut mpsc::UnboundedReceiver<Started>) -> Started {
    tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap()
        .unwrap()
}

pub struct ControlledExecutor {
    pub manager: Arc<JobManager>,
    pub started: mpsc::UnboundedSender<Started>,
}

#[async_trait::async_trait]
impl JobExecutorPort for ControlledExecutor {
    fn validate(&self, descriptor: &ExecutionDescriptor) -> Result<(), PortError> {
        descriptor.validate()
    }
    async fn execute(
        &self,
        execution: QueuedExecution,
        context: ExecutionContext,
    ) -> RuntimeTaskOutcome {
        let id = execution.job.id().clone();
        let (finish, finished) = oneshot::channel();
        let (cleanup, cleaned) = oneshot::channel();
        self.started
            .send(Started {
                id: id.clone(),
                finish,
                cleanup,
            })
            .unwrap();
        let success = tokio::select! {
            result = finished => result.unwrap_or(true),
            _ = context.cancellation.cancelled() => {
                let _ = cleaned.await;
                return RuntimeTaskOutcome::Cancelled;
            }
        };
        let result = if success {
            self.manager.complete_job(&id).await
        } else {
            self.manager
                .fail_job(
                    &id,
                    "FIXTURE_FAILURE".into(),
                    "Fixture failed".into(),
                    false,
                )
                .await
        };
        if result.is_err() {
            RuntimeTaskOutcome::RecoveryRequired
        } else if success {
            RuntimeTaskOutcome::Completed
        } else {
            RuntimeTaskOutcome::ApplicationFailed
        }
    }
}
