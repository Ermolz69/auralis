use super::diagnostic::{DiagnosticKind, DiagnosticLevel, DiagnosticSink, ProcessDiagnostic};
use crate::TracingShutdownOutcome;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
use tracing_appender::non_blocking::ErrorCounter;

pub(super) struct HealthSampler {
    stop: mpsc::Sender<()>,
    task: Option<std::thread::JoinHandle<()>>,
}

impl HealthSampler {
    pub(super) fn start(
        queues: Vec<ErrorCounter>,
        sink: Arc<dyn DiagnosticSink>,
        period: Duration,
    ) -> std::io::Result<Self> {
        let (stop, receiver) = mpsc::channel();
        let task = std::thread::Builder::new()
            .name("diagnostic-health".into())
            .spawn(move || {
                let mut previous = (0, 0);
                while matches!(
                    receiver.recv_timeout(period),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    let dropped = super::bounded_writer::DROPPED_EVENTS
                        .load(std::sync::atomic::Ordering::Relaxed)
                        .saturating_add(
                            queues
                                .iter()
                                .map(|queue| queue.dropped_lines() as u64)
                                .fold(0, u64::saturating_add),
                        );
                    let failed = super::bounded_writer::WRITE_FAILURES
                        .load(std::sync::atomic::Ordering::Relaxed);
                    for (kind, count, before) in [
                        (DiagnosticKind::BufferOverflow, dropped, previous.0),
                        (DiagnosticKind::RuntimeLogWriteFailed, failed, previous.1),
                    ] {
                        if count > before {
                            sink.emit(ProcessDiagnostic {
                                level: DiagnosticLevel::Warning,
                                kind,
                                count: Some(count - before),
                                os_code: None,
                                fallback: None,
                            });
                        }
                    }
                    previous = (dropped, failed);
                }
            })?;
        Ok(Self {
            stop,
            task: Some(task),
        })
    }

    pub(super) fn request_stop(&self) {
        let _ = self.stop.send(());
    }

    pub(super) fn stop_until(mut self, deadline: Instant) -> TracingShutdownOutcome {
        self.request_stop();
        let Some(task) = self.task.take() else {
            return TracingShutdownOutcome::NotOwned;
        };
        while !task.is_finished() {
            if Instant::now() >= deadline {
                return TracingShutdownOutcome::TimedOut;
            }
            std::thread::sleep(
                Duration::from_millis(1).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
        if task.join().is_ok() {
            TracingShutdownOutcome::Flushed
        } else {
            TracingShutdownOutcome::Failed
        }
    }
}

impl Drop for HealthSampler {
    fn drop(&mut self) {
        self.request_stop();
    }
}
