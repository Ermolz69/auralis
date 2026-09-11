use super::{
    diagnostic::{DiagnosticKind, DiagnosticLevel, DiagnosticSink, ProcessDiagnostic},
    health::HealthSampler,
    init::TracingMode,
    shutdown::TracingShutdownReport,
    sink::OwnedSink,
};
use crate::TracingShutdownOutcome;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub struct TracingGuard {
    pub(super) file: Option<OwnedSink>,
    pub(super) console: Option<OwnedSink>,
    pub(super) sampler: Option<HealthSampler>,
    pub active_mode: TracingMode,
    pub(super) diagnostic: Arc<dyn DiagnosticSink>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(sampler) = &self.sampler {
            sampler.request_stop();
        }
    }
}

impl TracingGuard {
    pub fn shutdown(mut self, timeout: Duration) -> TracingShutdownReport {
        let deadline = Instant::now() + timeout;
        let sampler = self
            .sampler
            .take()
            .map(|sampler| sampler.stop_until(deadline))
            .unwrap_or(if self.active_mode == TracingMode::ExistingSubscriber {
                TracingShutdownOutcome::NotOwned
            } else {
                TracingShutdownOutcome::Failed
            });
        let file = self
            .file
            .as_mut()
            .map(|sink| sink.stop_until(deadline))
            .unwrap_or(TracingShutdownOutcome::NotOwned);
        tracing::info!(event_name = "shutdown_sink_completed", sink = "file", outcome = ?file);
        if !file.is_graceful() {
            self.diagnostic.emit(ProcessDiagnostic {
                level: DiagnosticLevel::Warning,
                kind: DiagnosticKind::TracingFlushTimedOut,
                os_code: None,
                count: None,
                fallback: None,
            });
        }
        let console = self
            .console
            .as_mut()
            .map(|sink| sink.stop_until(deadline))
            .unwrap_or(TracingShutdownOutcome::NotOwned);
        TracingShutdownReport {
            file,
            console,
            sampler,
        }
    }
}
