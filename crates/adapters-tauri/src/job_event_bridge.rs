use crate::event_publisher::FrontendJobEventPublisher;
use application::services::job_lifecycle_coordinator::JobLifecycleCoordinator;
use ports::job_scheduler::JobLifecycleEvent;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

struct JobLifecycleWorker<P>
where
    P: FrontendJobEventPublisher + 'static,
{
    publisher: P,
    coordinator: Arc<JobLifecycleCoordinator>,
    receiver: mpsc::UnboundedReceiver<JobLifecycleEvent>,
    shutdown_rx: oneshot::Receiver<()>,
}

impl<P> JobLifecycleWorker<P>
where
    P: FrontendJobEventPublisher + 'static,
{
    async fn run(mut self) {
        loop {
            tokio::select! {
                _ = &mut self.shutdown_rx => {
                    tracing::info!("JobLifecycleWorker: shutdown signal received, stopping");
                    break;
                }
                event_opt = self.receiver.recv() => {
                    match event_opt {
                        Some(event) => {
                            if let Err(e) = self.publisher.publish_job_event(&event) {
                                tracing::error!(error = ?e, "failed to publish job event to frontend");
                            }

                            if let Err(e) = self.coordinator.handle(event).await {
                                tracing::error!(error = ?e, "failed to handle job lifecycle event");
                            }
                        }
                        None => {
                            tracing::info!("JobLifecycleWorker: all senders dropped, stopping");
                            break;
                        }
                    }
                }
            }
        }
    }
}

pub struct JobEventBridgeHandle {
    shutdown_tx: oneshot::Sender<()>,
    worker_handle: JoinHandle<()>,
}

impl JobEventBridgeHandle {
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.worker_handle.await;
    }
}

pub struct TauriJobEventBridge {
    tx: mpsc::UnboundedSender<JobLifecycleEvent>,
    handle: Option<JobEventBridgeHandle>,
}

impl TauriJobEventBridge {
    pub fn new<P>(publisher: P, coordinator: Arc<JobLifecycleCoordinator>) -> Self
    where
        P: FrontendJobEventPublisher + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let worker = JobLifecycleWorker {
            publisher,
            coordinator,
            receiver: rx,
            shutdown_rx,
        };

        // Using tokio::spawn which is the standard async_runtime backend
        let worker_handle = tokio::spawn(async move {
            worker.run().await;
        });

        let bridge_handle = JobEventBridgeHandle {
            shutdown_tx,
            worker_handle,
        };

        Self {
            tx,
            handle: Some(bridge_handle),
        }
    }

    pub fn emitter(&self) -> Arc<dyn Fn(JobLifecycleEvent) + Send + Sync> {
        let tx = self.tx.clone();
        Arc::new(move |event: JobLifecycleEvent| {
            if let Err(e) = tx.send(event) {
                tracing::error!(error = ?e, "failed to send job lifecycle event to worker channel");
            }
        })
    }

    pub fn take_handle(&mut self) -> Option<JobEventBridgeHandle> {
        self.handle.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{JobId, JobProgress, JobStatus};
    use ports::error::PortError;
    use std::sync::{Arc, Mutex};
    use tokio::time::{Duration, sleep};

    #[derive(Clone)]
    struct MockFrontendPublisher {
        events: Arc<Mutex<Vec<JobLifecycleEvent>>>,
        fail_next: Arc<Mutex<bool>>,
    }

    impl MockFrontendPublisher {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(vec![])),
                fail_next: Arc::new(Mutex::new(false)),
            }
        }
    }

    impl FrontendJobEventPublisher for MockFrontendPublisher {
        fn publish_job_event(&self, event: &JobLifecycleEvent) -> Result<(), PortError> {
            let mut fail = self.fail_next.lock().unwrap();
            if *fail {
                *fail = false;
                return Err(PortError::Unexpected {
                    message: "frontend pub failure".into(),
                });
            }
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_bridge_event_ordering_and_shutdown() {
        let coordinator = Arc::new(JobLifecycleCoordinator::new());

        let frontend_pub = MockFrontendPublisher::new();
        let mut bridge = TauriJobEventBridge::new(frontend_pub.clone(), coordinator);

        let emitter = bridge.emitter();

        let event1 = JobLifecycleEvent {
            stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
            job_id: JobId::new(),
            project_id: None,
            status: JobStatus::Running,
            progress: JobProgress::initializing(),
            error: None,
        };

        let event2 = JobLifecycleEvent {
            stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
            job_id: JobId::new(),
            project_id: None,
            status: JobStatus::Completed,
            progress: JobProgress::initializing(),
            error: None,
        };

        // Send two events
        emitter(event1.clone());
        emitter(event2.clone());

        // Wait a bit for processing
        sleep(Duration::from_millis(50)).await;

        let emitted = frontend_pub.events.lock().unwrap().clone();
        assert_eq!(emitted.len(), 2);
        assert_eq!(emitted[0].status, JobStatus::Running);
        assert_eq!(emitted[1].status, JobStatus::Completed);

        // Test shutdown
        let handle = bridge.take_handle().unwrap();
        handle.shutdown().await;

        // Try emitting after shutdown (should log error, but not panic)
        let event3 = JobLifecycleEvent {
            stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
            job_id: JobId::new(),
            project_id: None,
            status: JobStatus::Failed,
            progress: JobProgress::initializing(),
            error: None,
        };
        emitter(event3);
    }

    #[tokio::test]
    async fn test_bridge_processing_after_failure() {
        let coordinator = Arc::new(JobLifecycleCoordinator::new());

        let frontend_pub = MockFrontendPublisher::new();
        let mut bridge = TauriJobEventBridge::new(frontend_pub.clone(), coordinator);

        let emitter = bridge.emitter();

        // Make the first publish fail
        *frontend_pub.fail_next.lock().unwrap() = true;

        let event1 = JobLifecycleEvent {
            stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
            job_id: JobId::new(),
            project_id: None,
            status: JobStatus::Running,
            progress: JobProgress::initializing(),
            error: None,
        };

        let event2 = JobLifecycleEvent {
            stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
            job_id: JobId::new(),
            project_id: None,
            status: JobStatus::Completed,
            progress: JobProgress::initializing(),
            error: None,
        };

        // Send events
        emitter(event1);
        emitter(event2);

        sleep(Duration::from_millis(50)).await;

        // The first event failed to publish, but the worker should have continued to process the second event
        let emitted = frontend_pub.events.lock().unwrap().clone();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].status, JobStatus::Completed);

        let handle = bridge.take_handle().unwrap();
        handle.shutdown().await;
    }
}
