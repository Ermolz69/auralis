use super::config::ObservabilityConfig;
use std::fs;
use std::io::stderr;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub struct TracingGuard {
    _guard: Option<WorkerGuard>,
}

pub fn init(config: ObservabilityConfig) -> TracingGuard {
    let mut guard = None;

    // Use a custom format for the terminal
    let console_layer = fmt::layer()
        .with_writer(stderr)
        .with_ansi(true)
        .with_target(false);

    // Default env filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Some(log_dir) = config.log_dir {
        // Ensure log dir exists
        if let Err(e) = fs::create_dir_all(&log_dir) {
            use std::io::Write;
            let _ = writeln!(
                std::io::stderr(),
                "WARNING: Failed to create log directory: {}",
                e
            );

            // Console-only fallback
            let _ = tracing_subscriber::registry()
                .with(env_filter)
                .with(console_layer)
                .try_init();

            return TracingGuard { _guard: None };
        }

        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("auralis.log")
            .max_log_files(30)
            .build(log_dir)
            .unwrap_or_else(|e| {
                panic!("Failed to create file appender: {}", e);
            });

        // Using standard non_blocking builder is lossy by default.
        // We can create a non_blocking worker builder with lossy(false) if desired,
        // but currently tracing_appender::non_blocking defaults to non-lossy bounded channel
        // in tracing-appender 0.2 if we don't specify it, actually it's lossy by default in some versions.
        // Let's use builder to be explicit:
        let (non_blocking, worker_guard) =
            tracing_appender::non_blocking::NonBlockingBuilder::default()
                .lossy(false)
                .finish(file_appender);

        guard = Some(worker_guard);

        let file_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(true);

        let _ = tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .try_init();
    } else {
        // Console-only fallback
        let _ = tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .try_init();
    }

    TracingGuard { _guard: guard }
}
