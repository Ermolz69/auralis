use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing_subscriber::fmt::MakeWriter;

pub(super) const MAX_EVENT_BYTES: usize = 32 * 1024;
pub(super) const MAX_QUEUED_EVENTS: usize = 128;
pub(super) static DROPPED_EVENTS: AtomicU64 = AtomicU64::new(0);
pub(super) static WRITE_FAILURES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub(super) struct BoundedWriter<M>(pub M);

pub(super) struct EventWriter<W: Write> {
    inner: W,
    bytes: Vec<u8>,
    oversized: bool,
}

impl<'a, M: MakeWriter<'a>> MakeWriter<'a> for BoundedWriter<M> {
    type Writer = EventWriter<M::Writer>;
    fn make_writer(&'a self) -> Self::Writer {
        EventWriter {
            inner: self.0.make_writer(),
            bytes: Vec::new(),
            oversized: false,
        }
    }
}

impl<W: Write> Write for EventWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.oversized {
            if bytes.len() > MAX_EVENT_BYTES.saturating_sub(self.bytes.len()) {
                self.oversized = true;
                self.bytes.clear();
            } else {
                self.bytes.extend_from_slice(bytes);
            }
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<W: Write> Drop for EventWriter<W> {
    fn drop(&mut self) {
        if self.oversized {
            DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
        } else if self.inner.write_all(&self.bytes).is_err() {
            WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn oversized_json_is_dropped_as_a_whole_event() {
        let capture = Capture(Arc::new(Mutex::new(Vec::new())));
        let writer = BoundedWriter(|| capture.clone());
        let mut event = writer.make_writer();
        event.write_all(b"{\"message\":\"").unwrap();
        event.write_all(&vec![b'x'; MAX_EVENT_BYTES]).unwrap();
        event.write_all(b"\"}\n").unwrap();
        assert!(event.bytes.len() <= MAX_EVENT_BYTES);
        drop(event);
        assert!(capture.0.lock().unwrap().is_empty());
        writer.make_writer().write_all(b"{\"ok\":true}\n").unwrap();
        assert_eq!(&*capture.0.lock().unwrap(), b"{\"ok\":true}\n");
    }

    #[derive(Clone, Copy)]
    struct Unavailable;
    impl Write for Unavailable {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn runtime_io_failure_is_counted_without_escaping_to_the_caller() {
        let before = WRITE_FAILURES.load(Ordering::Relaxed);
        let writer = BoundedWriter(|| Unavailable);
        writer
            .make_writer()
            .write_all(b"{\"event\":\"committed\"}\n")
            .unwrap();
        assert!(WRITE_FAILURES.load(Ordering::Relaxed) > before);
    }
}
