use ports::events::AppEventPublisher;
use std::time::Duration;

pub(super) async fn publish_terminal_notifications(
    publisher: &dyn AppEventPublisher,
    project_id: &str,
    job_id: &str,
    transcript_ready: bool,
) {
    if transcript_ready {
        publish_with_retry("transcript-ready", || {
            publisher.publish_transcript_ready(project_id, job_id)
        })
        .await;
    }
    publish_with_retry("project-updated", || {
        publisher.publish_project_updated(project_id)
    })
    .await;
}

async fn publish_with_retry<F, Fut>(event: &'static str, mut publish: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), ports::error::PortError>>,
{
    for attempt in 1..=2 {
        if matches!(
            tokio::time::timeout(Duration::from_millis(250), publish()).await,
            Ok(Ok(()))
        ) {
            return;
        }
        tracing::warn!(
            event,
            attempt,
            code = "TerminalNotificationFailed",
            fallback = "snapshot_refresh",
            "terminal notification unavailable after commit"
        );
    }
}
