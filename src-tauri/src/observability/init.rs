use super::config::{LogDestination, LogFormat, ObservabilityConfig, OverflowPolicy};
use std::fs;
use std::io::{stderr, Write};
use tracing_appender::non_blocking::{ErrorCounter, WorkerGuard};

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[derive(Debug, Clone, PartialEq)]
pub enum TracingMode {
    FileAndConsole,
    ConsoleOnly,
    ExistingSubscriber,
}

pub struct TracingGuard {
    worker_guard: Option<WorkerGuard>,
    pub active_mode: TracingMode,
    error_counter: Option<ErrorCounter>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(guard) = self.worker_guard.take() {
            // Drop worker guard explicitly to flush remaining log lines
            drop(guard);
        }

        if let Some(ref counter) = self.error_counter {
            let dropped = counter.dropped_lines();
            if dropped > 0 {
                let _ = writeln!(
                    stderr().lock(),
                    "WARNING: Tracing dropped {} log lines due to full buffer",
                    dropped
                );
            }
        }
    }
}

pub trait DiagnosticSink {
    fn emit_warning(&self, msg: &str);
}

pub struct StderrDiagnosticSink;

impl DiagnosticSink for StderrDiagnosticSink {
    fn emit_warning(&self, msg: &str) {
        let _ = writeln!(stderr().lock(), "WARNING: {}", msg);
    }
}

pub fn init(config: ObservabilityConfig, sink: &dyn DiagnosticSink) -> TracingGuard {
    let mut warnings = Vec::new();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|e| {
        if std::env::var("RUST_LOG").is_ok() {
            warnings.push(format!(
                "Invalid RUST_LOG environment variable ({}), falling back to default.",
                e
            ));
        }
        config.default_filter
    });

    let console_base = fmt::layer().with_writer(stderr).with_ansi(true);
    let console_layer = match config.console_format {
        LogFormat::Pretty => console_base.pretty().boxed(),
        LogFormat::Compact => console_base.compact().boxed(),
        LogFormat::Json => console_base.json().boxed(),
    };

    let mut file_layer = None;
    let mut guard = None;
    let mut error_counter = None;
    let mut use_console_only = false;

    match config.log_dir {
        LogDestination::Disabled => {
            use_console_only = true;
        }
        LogDestination::Unavailable(kind) => {
            warnings.push(format!("Log directory unavailable: {:?}", kind));
            use_console_only = true;
        }
        LogDestination::Directory(log_dir) => {
            if let Err(e) = fs::create_dir_all(&log_dir) {
                warnings.push(format!("Failed to create log directory: {}", e));
                use_console_only = true;
            } else {
                match tracing_appender::rolling::Builder::new()
                    .rotation(config.rotation)
                    .filename_prefix("auralis.log")
                    .max_log_files(config.max_log_files)
                    .build(&log_dir)
                {
                    Ok(appender) => {
                        let is_lossy = config.overflow_policy == OverflowPolicy::DropWhenFull;
                        let (non_blocking, worker_guard) =
                            tracing_appender::non_blocking::NonBlockingBuilder::default()
                                .lossy(is_lossy)
                                .buffered_lines_limit(config.buffer_capacity)
                                .finish(appender);

                        if is_lossy {
                            error_counter = Some(non_blocking.error_counter().clone());
                        }

                        guard = Some(worker_guard);

                        let file_base = fmt::layer().with_writer(non_blocking).with_ansi(false);
                        let fl = match config.file_format {
                            LogFormat::Pretty => file_base.pretty().boxed(),
                            LogFormat::Compact => file_base.compact().boxed(),
                            LogFormat::Json => file_base.json().boxed(),
                        };
                        file_layer = Some(fl);
                    }
                    Err(e) => {
                        warnings.push(format!("Failed to create file appender: {}", e));
                        use_console_only = true;
                    }
                }
            }
        }
    }

    let mut active_mode = if use_console_only {
        TracingMode::ConsoleOnly
    } else {
        TracingMode::FileAndConsole
    };

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    let init_result = if let Some(fl) = file_layer {
        registry.with(fl).try_init()
    } else {
        registry.try_init()
    };

    if init_result.is_err() {
        warnings
            .push("Global subscriber already set. Existing subscriber will be used.".to_string());
        active_mode = TracingMode::ExistingSubscriber;
        // Do not hold onto the worker guard if we failed to set the subscriber
        guard = None;
        error_counter = None;
    }

    let mut unique_warnings = Vec::new();
    for warning in warnings {
        if !unique_warnings.contains(&warning) {
            unique_warnings.push(warning.clone());
            sink.emit_warning(&warning);
        }
    }

    TracingGuard {
        worker_guard: guard,
        active_mode,
        error_counter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    struct MockSink {
        warnings: Arc<Mutex<Vec<String>>>,
    }

    impl DiagnosticSink for MockSink {
        fn emit_warning(&self, msg: &str) {
            self.warnings.lock().unwrap().push(msg.to_string());
        }
    }

    #[test]
    fn test_init_disabled() {
        let warnings = Arc::new(Mutex::new(Vec::new()));
        let sink = MockSink {
            warnings: warnings.clone(),
        };
        let config = ObservabilityConfig {
            log_dir: LogDestination::Disabled,
            default_filter: tracing_subscriber::EnvFilter::new("info"),
            buffer_capacity: 100,
            overflow_policy: OverflowPolicy::Backpressure,
            max_log_files: 5,
            rotation: tracing_appender::rolling::Rotation::DAILY,
            console_format: LogFormat::Compact,
            file_format: LogFormat::Json,
        };

        let guard = init(config, &sink);

        // Either ConsoleOnly or ExistingSubscriber if already initialized in this process
        assert!(
            guard.active_mode == TracingMode::ConsoleOnly
                || guard.active_mode == TracingMode::ExistingSubscriber
        );
    }

    #[test]
    fn test_init_invalid_dir() {
        let warnings = Arc::new(Mutex::new(Vec::new()));
        let sink = MockSink {
            warnings: warnings.clone(),
        };

        #[cfg(unix)]
        let path = std::path::PathBuf::from("/dev/null/invalid");
        #[cfg(windows)]
        let path = std::path::PathBuf::from("Z:\\invalid\\dir<>"); // invalid path on windows

        let config = ObservabilityConfig {
            log_dir: LogDestination::Directory(path),
            default_filter: tracing_subscriber::EnvFilter::new("info"),
            buffer_capacity: 100,
            overflow_policy: OverflowPolicy::Backpressure,
            max_log_files: 5,
            rotation: tracing_appender::rolling::Rotation::DAILY,
            console_format: LogFormat::Compact,
            file_format: LogFormat::Json,
        };

        let guard = init(config, &sink);
        let _warn_logs = warnings.lock().unwrap();
        assert!(
            guard.active_mode == TracingMode::ConsoleOnly
                || guard.active_mode == TracingMode::ExistingSubscriber
        );
        // Note: There might be other warnings if it's already initialized, but at least one should mention the failure.
    }

    #[test]
    fn test_init_valid_dir() {
        let warnings = Arc::new(Mutex::new(Vec::new()));
        let sink = MockSink {
            warnings: warnings.clone(),
        };
        let dir = tempdir().unwrap();

        let config = ObservabilityConfig {
            log_dir: LogDestination::Directory(dir.path().to_path_buf()),
            default_filter: tracing_subscriber::EnvFilter::new("info"),
            buffer_capacity: 100,
            overflow_policy: OverflowPolicy::Backpressure,
            max_log_files: 5,
            rotation: tracing_appender::rolling::Rotation::DAILY,
            console_format: LogFormat::Compact,
            file_format: LogFormat::Json,
        };

        let guard = init(config, &sink);
        assert!(
            guard.active_mode == TracingMode::FileAndConsole
                || guard.active_mode == TracingMode::ExistingSubscriber
        );
    }
}
