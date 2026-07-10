use crate::TauriEventPublisher;
use application::services::job_lifecycle_coordinator::JobLifecycleCoordinator;
use ports::events::AppEventPublisher;
use ports::job_scheduler::JobLifecycleEvent;
use ports::repository::ProjectRepository;
use std::sync::Arc;
use tauri::async_runtime::JoinHandle;
use tokio::sync::mpsc;

struct JobLifecycleWorker<R, E>
where
    R: ProjectRepository + Clone + Send + Sync + 'static,
    E: AppEventPublisher + Clone + Send + Sync + 'static,
{
    publisher: TauriEventPublisher,
    coordinator: Arc<JobLifecycleCoordinator<R, E>>,
    receiver: mpsc::UnboundedReceiver<JobLifecycleEvent>,
}

impl<R, E> JobLifecycleWorker<R, E>
where
    R: ProjectRepository + Clone + Send + Sync + 'static,
    E: AppEventPublisher + Clone + Send + Sync + 'static,
{
    async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            self.publisher.publish_job_event(&event);

            if let Err(e) = self.coordinator.handle(event).await {
                eprintln!("JobLifecycleWorker: failed to handle event: {:?}", e);
            }
        }
    }
}

pub struct TauriJobEventBridge {
    tx: mpsc::UnboundedSender<JobLifecycleEvent>,
    shutdown_handle: JoinHandle<()>,
}

impl TauriJobEventBridge {
    pub fn new<R, E>(
        publisher: TauriEventPublisher,
        coordinator: Arc<JobLifecycleCoordinator<R, E>>,
    ) -> Self
    where
        R: ProjectRepository + Clone + Send + Sync + 'static,
        E: AppEventPublisher + Clone + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();

        let worker = JobLifecycleWorker {
            publisher,
            coordinator,
            receiver: rx,
        };

        let shutdown_handle = tauri::async_runtime::spawn(async move {
            worker.run().await;
        });

        Self {
            tx,
            shutdown_handle,
        }
    }

    pub fn emitter(&self) -> Arc<dyn Fn(JobLifecycleEvent) + Send + Sync> {
        let tx = self.tx.clone();
        Arc::new(move |event: JobLifecycleEvent| {
            let _ = tx.send(event);
        })
    }

    pub fn shutdown_handle(self) -> JoinHandle<()> {
        self.shutdown_handle
    }
}
