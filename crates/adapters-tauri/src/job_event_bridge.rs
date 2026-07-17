#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::event_publisher::FrontendJobEventPublisher;
use application::services::job_lifecycle_coordinator::JobLifecycleCoordinator;
use ports::job_scheduler::JobLifecycleEvent;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;

pub const JOB_EVENT_CHANNEL_CAPACITY: usize = 256;

struct JobLifecycleWorker<P>
where
    P: FrontendJobEventPublisher + 'static,
{
    publisher: P,
    coordinator: Arc<JobLifecycleCoordinator>,
    receiver: broadcast::Receiver<JobLifecycleEvent>,
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
                event_result = self.receiver.recv() => {
                    match event_result {
                        Ok(event) => {
                            if let Err(_e) = self.publisher.publish_job_event(&event) {
                                tracing::error!(
                                    error = %common::observability::redaction::DiagnosticError {
                                        kind: "FrontendPublishError",
                                        code: None,
                                        retryable: false,
                                    },
                                    "failed to publish job event to frontend"
                                );
                            }

                            if let Err(_e) = self.coordinator.handle(event).await {
                                tracing::error!(
                                    error = %common::observability::redaction::DiagnosticError {
                                        kind: "LifecycleCoordinatorError",
                                        code: None,
                                        retryable: false,
                                    },
                                    "failed to handle job lifecycle event"
                                );
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(skipped_events = skipped, "JobLifecycleWorker lagged behind, skipped events. Publishing invalidated event.");
                            if let Err(_e) = self.publisher.publish_invalidated() {
                                tracing::error!(
                                    error = %common::observability::redaction::DiagnosticError {
                                        kind: "FrontendPublishError",
                                        code: None,
                                        retryable: false,
                                    },
                                    "failed to publish invalidated event to frontend"
                                );
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
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
    shutdown_tx: Option<oneshot::Sender<()>>,
    worker_handle: Option<JoinHandle<()>>,
}

impl JobEventBridgeHandle {
    pub fn into_shutdown_parts(mut self) -> (Option<oneshot::Sender<()>>, Option<JoinHandle<()>>) {
        (self.shutdown_tx.take(), self.worker_handle.take())
    }
}

pub struct TauriJobEventBridge {
    tx: broadcast::Sender<JobLifecycleEvent>,
    handle: Option<JobEventBridgeHandle>,
}

impl TauriJobEventBridge {
    pub fn new<P>(publisher: P, coordinator: Arc<JobLifecycleCoordinator>) -> Self
    where
        P: FrontendJobEventPublisher + 'static,
    {
        let (tx, rx) = broadcast::channel(JOB_EVENT_CHANNEL_CAPACITY);
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
            shutdown_tx: Some(shutdown_tx),
            worker_handle: Some(worker_handle),
        };

        Self {
            tx,
            handle: Some(bridge_handle),
        }
    }

    pub fn emitter(&self) -> Arc<dyn Fn(JobLifecycleEvent) + Send + Sync> {
        let tx = self.tx.clone();
        Arc::new(move |event: JobLifecycleEvent| {
            if let Err(_e) = tx.send(event) {
                tracing::error!(
                    error = %common::observability::redaction::DiagnosticError {
                        kind: "ChannelSendError",
                        code: None,
                        retryable: false,
                    },
                    "failed to send job lifecycle event to worker channel"
                );
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
    use chrono;
    use domain::job::{JobId, JobProgress, JobStatus};
    use ports::error::PortError;
    use std::sync::{Arc, Mutex};
    use tokio::time::{Duration, sleep};

    #[derive(Clone)]
    struct MockFrontendPublisher {
        events: Arc<Mutex<Vec<JobLifecycleEvent>>>,
        fail_next: Arc<Mutex<bool>>,
        invalidated_calls: Arc<Mutex<usize>>,
    }

    impl MockFrontendPublisher {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(vec![])),
                fail_next: Arc::new(Mutex::new(false)),
                invalidated_calls: Arc::new(Mutex::new(0)),
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

        fn publish_invalidated(&self) -> Result<(), PortError> {
            *self.invalidated_calls.lock().unwrap() += 1;
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
            kind: ports::job_scheduler::JobLifecycleEventKind::Progressed,
            job: ports::job_scheduler::ScheduledJob {
                id: JobId::new(),
                revision: 1,
                title: "Test".to_string(),
                project_id: None,
                status: JobStatus::Running,
                stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
                progress: JobProgress::initializing(),
                error: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        };

        let event2 = JobLifecycleEvent {
            kind: ports::job_scheduler::JobLifecycleEventKind::Completed,
            job: ports::job_scheduler::ScheduledJob {
                id: JobId::new(),
                revision: 2,
                title: "Test".to_string(),
                project_id: None,
                status: JobStatus::Completed,
                stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
                progress: JobProgress::initializing(),
                error: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        };

        // Send two events
        emitter(event1.clone());
        emitter(event2.clone());

        // Wait a bit for processing
        sleep(Duration::from_millis(50)).await;

        let emitted = frontend_pub.events.lock().unwrap().clone();
        assert_eq!(emitted.len(), 2);
        assert_eq!(emitted[0].job.status, JobStatus::Running);
        assert_eq!(emitted[1].job.status, JobStatus::Completed);

        // Test shutdown
        let handle = bridge.take_handle().unwrap();
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx {
            let _ = tx.send(());
        }
        if let Some(task) = task {
            let _ = task.await;
        }

        // Try emitting after shutdown (should log error, but not panic)
        let event3 = JobLifecycleEvent {
            kind: ports::job_scheduler::JobLifecycleEventKind::Failed,
            job: ports::job_scheduler::ScheduledJob {
                id: JobId::new(),
                revision: 3,
                title: "Test".to_string(),
                project_id: None,
                status: JobStatus::Failed,
                stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
                progress: JobProgress::initializing(),
                error: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
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
            kind: ports::job_scheduler::JobLifecycleEventKind::Started,
            job: ports::job_scheduler::ScheduledJob {
                id: JobId::new(),
                revision: 1,
                title: "Test".to_string(),
                project_id: None,
                status: JobStatus::Running,
                stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
                progress: JobProgress::initializing(),
                error: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        };

        let event2 = JobLifecycleEvent {
            kind: ports::job_scheduler::JobLifecycleEventKind::Completed,
            job: ports::job_scheduler::ScheduledJob {
                id: JobId::new(),
                revision: 2,
                title: "Test".to_string(),
                project_id: None,
                status: JobStatus::Completed,
                stage: Some(domain::dubbing::DubbingPipelineStage::TranslateTranscript),
                progress: JobProgress::initializing(),
                error: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        };

        // Send events
        emitter(event1);
        emitter(event2);

        sleep(Duration::from_millis(50)).await;

        // The first event failed to publish, but the worker should have continued to process the second event
        let emitted = frontend_pub.events.lock().unwrap().clone();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].job.status, JobStatus::Completed);

        let handle = bridge.take_handle().unwrap();
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx {
            let _ = tx.send(());
        }
        if let Some(task) = task {
            let _ = task.await;
        }
    }

    #[tokio::test]
    async fn test_bridge_lagged_receiver() {
        let coordinator = Arc::new(JobLifecycleCoordinator::new());
        let frontend_pub = MockFrontendPublisher::new();

        // We use a small capacity channel for testing lagged
        let (tx, rx) = broadcast::channel(2);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let worker = JobLifecycleWorker {
            publisher: frontend_pub.clone(),
            coordinator,
            receiver: rx,
            shutdown_rx,
        };

        let worker_handle = tokio::spawn(async move {
            worker.run().await;
        });

        // Fill the channel and overflow it
        for _ in 0..5 {
            let event = JobLifecycleEvent {
                kind: ports::job_scheduler::JobLifecycleEventKind::Progressed,
                job: ports::job_scheduler::ScheduledJob {
                    id: JobId::new(),
                    revision: 1,
                    title: "Test".to_string(),
                    project_id: None,
                    status: JobStatus::Running,
                    stage: None,
                    progress: JobProgress::initializing(),
                    error: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                },
            };
            let _ = tx.send(event);
        }

        // Give worker time to process the lagged error
        sleep(Duration::from_millis(50)).await;

        let invalidations = *frontend_pub.invalidated_calls.lock().unwrap();
        assert!(
            invalidations > 0,
            "Expected at least one invalidated event to be published"
        );

        let _ = shutdown_tx.send(());
        let _ = worker_handle.await;
    }
}
