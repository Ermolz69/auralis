use super::notifications::publish_terminal_notifications;
use async_trait::async_trait;
use ports::{error::PortError, events::AppEventPublisher};
use std::sync::atomic::{AtomicUsize, Ordering};

struct FailingPublisher {
    projects: AtomicUsize,
    transcripts: AtomicUsize,
    always_fail: bool,
}
#[async_trait]
impl AppEventPublisher for FailingPublisher {
    async fn publish_project_updated(&self, _: &str) -> Result<(), PortError> {
        let attempt = self.projects.fetch_add(1, Ordering::SeqCst);
        if !self.always_fail && attempt > 0 {
            Ok(())
        } else {
            Err(PortError::AlreadyStopped)
        }
    }
    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), PortError> {
        self.transcripts.fetch_add(1, Ordering::SeqCst);
        Err(PortError::AlreadyStopped)
    }
}
#[tokio::test]
async fn failed_transcript_does_not_skip_project_retry_or_fail_committed_operation() {
    for always_fail in [true, false] {
        let publisher = FailingPublisher {
            projects: AtomicUsize::new(0),
            transcripts: AtomicUsize::new(0),
            always_fail,
        };
        publish_terminal_notifications(&publisher, "project", "job", true).await;
        assert_eq!(publisher.projects.load(Ordering::SeqCst), 2);
        assert_eq!(publisher.transcripts.load(Ordering::SeqCst), 2);
    }
}
