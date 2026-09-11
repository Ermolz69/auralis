use super::diagnostic::{
    DiagnosticKind, DiagnosticLevel, DiagnosticSink, ProcessDiagnostic, StderrDiagnosticSink,
};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_appender::non_blocking::ErrorCounter;

pub(super) fn start_sampler(queue: ErrorCounter) {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let result = std::thread::Builder::new()
        .name("diagnostic-health".into())
        .spawn(move || {
            let mut previous = (0, 0);
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                let dropped = super::bounded_writer::DROPPED_EVENTS
                    .load(Ordering::Relaxed)
                    .saturating_add(queue.dropped_lines() as u64)
                    .saturating_add(
                        super::diagnostic::stderr_writer()
                            .error_counter()
                            .dropped_lines() as u64,
                    );
                let failed = super::bounded_writer::WRITE_FAILURES.load(Ordering::Relaxed);
                for (kind, count, before) in [
                    (DiagnosticKind::BufferOverflow, dropped, previous.0),
                    (DiagnosticKind::RuntimeLogWriteFailed, failed, previous.1),
                ] {
                    if count > before {
                        StderrDiagnosticSink.emit(ProcessDiagnostic {
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
        });
    if result.is_err() {
        STARTED.store(false, Ordering::Release);
    }
}
