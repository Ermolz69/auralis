use crate::TracingShutdownOutcome;
use std::{
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Instant,
};
use tracing_appender::non_blocking::{NonBlocking, NonBlockingBuilder, WorkerGuard};

pub struct OwnedSink {
    pub(crate) writer: NonBlocking,
    guard: Option<WorkerGuard>,
    finished: mpsc::Receiver<bool>,
}

impl OwnedSink {
    pub(crate) fn new(writer: impl Write + Send + 'static, capacity: usize, lossy: bool) -> Self {
        let (done, finished) = mpsc::channel();
        let writer = CheckedWriter {
            writer,
            failed: AtomicBool::new(false),
            done,
        };
        let (writer, guard) = NonBlockingBuilder::default()
            .lossy(lossy)
            .buffered_lines_limit(capacity.min(super::bounded_writer::MAX_QUEUED_EVENTS))
            .finish(writer);
        Self {
            writer,
            guard: Some(guard),
            finished,
        }
    }

    pub fn shutdown(mut self, timeout: std::time::Duration) -> TracingShutdownOutcome {
        self.stop_until(Instant::now() + timeout)
    }

    pub(super) fn stop_until(&mut self, deadline: Instant) -> TracingShutdownOutcome {
        let Some(guard) = self.guard.take() else {
            return TracingShutdownOutcome::NotOwned;
        };
        if !release_guard(guard) {
            return TracingShutdownOutcome::FlushThreadStartFailed;
        }
        match self
            .finished
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(true) => TracingShutdownOutcome::Flushed,
            Ok(false) => TracingShutdownOutcome::Failed,
            Err(mpsc::RecvTimeoutError::Timeout) => TracingShutdownOutcome::TimedOut,
            Err(mpsc::RecvTimeoutError::Disconnected) => TracingShutdownOutcome::Failed,
        }
    }
}

impl Drop for OwnedSink {
    fn drop(&mut self) {
        if let Some(guard) = self.guard.take() {
            release_guard(guard);
        }
    }
}

fn release_guard(guard: WorkerGuard) -> bool {
    let owner = Arc::new(Mutex::new(Some(guard)));
    let task_owner = owner.clone();
    if std::thread::Builder::new()
        .name("diagnostic-flush".into())
        .spawn(move || {
            drop(task_owner.lock().unwrap_or_else(|e| e.into_inner()).take());
        })
        .is_err()
    {
        // A failed spawn must not run the potentially blocking WorkerGuard drop on shutdown.
        std::mem::forget(owner);
        return false;
    }
    true
}

struct CheckedWriter<W: Write> {
    writer: W,
    failed: AtomicBool,
    done: mpsc::Sender<bool>,
}

impl<W: Write> Write for CheckedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.writer.write(bytes) {
            Ok(0) if !bytes.is_empty() => {
                self.record_failure();
                Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "Diagnostic sink made no write progress",
                ))
            }
            Ok(written) => Ok(written),
            Err(error) => {
                self.record_failure();
                Err(std::io::Error::new(
                    error.kind(),
                    "Diagnostic sink write failed",
                ))
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self.writer.flush() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.record_failure();
                Err(std::io::Error::new(
                    error.kind(),
                    "Diagnostic sink flush failed",
                ))
            }
        }
    }
}

impl<W: Write> CheckedWriter<W> {
    fn record_failure(&self) {
        self.failed.store(true, Ordering::Relaxed);
        super::bounded_writer::WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
    }
}

impl<W: Write> Drop for CheckedWriter<W> {
    fn drop(&mut self) {
        let _ = self.done.send(!self.failed.load(Ordering::Relaxed));
    }
}
