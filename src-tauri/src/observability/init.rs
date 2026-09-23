use super::config::{LogDestination, LogFormat, ObservabilityConfig, OverflowPolicy};
use super::diagnostic::{
    DiagnosticFallback, DiagnosticKind, DiagnosticLevel, DiagnosticSink, ProcessDiagnostic,
};
pub use super::guard::TracingGuard;
use std::sync::Arc;
use tracing_subscriber::{Layer, fmt, layer::SubscriberExt};

pub(crate) use super::environment::{ObservabilityEnvironment, RealObservabilityEnvironment};
pub use super::environment::{
    ObservabilityResourceError, ResolvedEnvFilter, SubscriberInstallError, TracingMode,
    resolve_env_filter,
};

pub fn init(config: ObservabilityConfig) -> TracingGuard {
    init_with_environment(config, None, &RealObservabilityEnvironment)
}

pub(crate) fn init_with_environment(
    config: ObservabilityConfig,
    injected_sink: Option<Arc<dyn DiagnosticSink>>,
    env: &dyn ObservabilityEnvironment,
) -> TracingGuard {
    if env.has_subscriber() {
        let sink =
            injected_sink.unwrap_or_else(|| Arc::new(super::diagnostic::TracingDiagnosticSink));
        sink.emit(ProcessDiagnostic {
            level: DiagnosticLevel::Warning,
            kind: DiagnosticKind::SubscriberAlreadyInstalled,
            os_code: None,
            count: None,
            fallback: Some(DiagnosticFallback::ExistingSubscriber),
        });
        return TracingGuard {
            file: None,
            console: None,
            sampler: None,
            active_mode: TracingMode::ExistingSubscriber,
            diagnostic: sink,
        };
    }

    let console = env.build_console();
    let sink = injected_sink.unwrap_or_else(|| {
        Arc::new(super::diagnostic::StderrDiagnosticSink(
            console.writer.clone(),
        ))
    });
    let mut use_console_only = false;
    let mut file_layer = None;
    let mut guard = None;

    match config.log_dir {
        LogDestination::Disabled => {
            use_console_only = true;
        }
        LogDestination::Unavailable(_) => {
            sink.emit(ProcessDiagnostic {
                level: DiagnosticLevel::Warning,
                kind: DiagnosticKind::LogDirectoryUnavailable,
                os_code: None,
                count: None,
                fallback: Some(DiagnosticFallback::ConsoleOnly),
            });
            use_console_only = true;
        }
        LogDestination::Directory(log_dir) => match env.create_directory(&log_dir) {
            Err(e) => {
                let os_code = match e {
                    ObservabilityResourceError::Io { os_code } => os_code,
                    _ => None,
                };
                sink.emit(ProcessDiagnostic {
                    level: DiagnosticLevel::Warning,
                    kind: DiagnosticKind::LogDirectoryUnavailable,
                    os_code,
                    count: None,
                    fallback: Some(DiagnosticFallback::ConsoleOnly),
                });
                use_console_only = true;
            }
            Ok(()) => {
                let is_lossy = config.settings.overflow_policy == OverflowPolicy::DropWhenFull;
                match env.build_non_blocking(
                    &log_dir,
                    config.settings.rotation.clone(),
                    config.settings.max_log_files,
                    config.settings.buffer_capacity,
                    is_lossy,
                ) {
                    Ok(owned) => {
                        let non_blocking = owned.writer.clone();
                        guard = Some(owned);

                        let file_base = fmt::layer()
                            .with_writer(super::bounded_writer::BoundedWriter(non_blocking))
                            .with_ansi(false);
                        let fl = match config.settings.file_format {
                            LogFormat::Pretty => file_base.pretty().boxed(),
                            LogFormat::Compact => file_base.compact().boxed(),
                            LogFormat::Json => file_base.json().boxed(),
                        };
                        file_layer = Some(fl);
                    }
                    Err(e) => {
                        let os_code = match e {
                            ObservabilityResourceError::Io { os_code } => os_code,
                            _ => None,
                        };
                        sink.emit(ProcessDiagnostic {
                            level: DiagnosticLevel::Warning,
                            kind: DiagnosticKind::FileAppenderUnavailable,
                            os_code,
                            count: None,
                            fallback: Some(DiagnosticFallback::ConsoleOnly),
                        });
                        use_console_only = true;
                    }
                }
            }
        },
    }

    let active_mode = if use_console_only {
        TracingMode::ConsoleOnly
    } else {
        TracingMode::FileAndConsole
    };

    let raw_rust_log = std::env::var("RUST_LOG").ok();
    let resolved = resolve_env_filter(raw_rust_log.as_deref(), &config.settings.default_filter);
    if resolved.used_default {
        sink.emit(ProcessDiagnostic {
            level: DiagnosticLevel::Warning,
            kind: DiagnosticKind::InvalidRustLog,
            os_code: None,
            count: None,
            fallback: None,
        });
    }

    let console_base = fmt::layer()
        .with_writer(super::bounded_writer::BoundedWriter(console.writer.clone()))
        .with_ansi(true);
    let console_layer = match config.settings.console_format {
        LogFormat::Pretty => console_base.pretty().boxed(),
        LogFormat::Compact => console_base.compact().boxed(),
        LogFormat::Json => console_base.json().boxed(),
    };

    let registry = tracing_subscriber::registry()
        .with(resolved.filter)
        .with(console_layer);

    let dispatch = if let Some(fl) = file_layer {
        tracing::Dispatch::new(registry.with(fl))
    } else {
        tracing::Dispatch::new(registry)
    };

    match env.set_global_default(dispatch) {
        Ok(()) => {
            let mut queues = vec![console.writer.error_counter()];
            if let Some(file) = &guard {
                queues.push(file.writer.error_counter());
            }
            let sampler = super::health::HealthSampler::start(
                queues,
                sink.clone(),
                std::time::Duration::from_secs(60),
            );
            if sampler.is_err() {
                sink.emit(ProcessDiagnostic {
                    level: DiagnosticLevel::Warning,
                    kind: DiagnosticKind::HealthSamplerUnavailable,
                    os_code: None,
                    count: None,
                    fallback: None,
                });
            }
            TracingGuard {
                file: guard,
                console: Some(console),
                sampler: sampler.ok(),
                active_mode,
                diagnostic: sink,
            }
        }
        Err(SubscriberInstallError::AlreadyInstalled) => {
            sink.emit(ProcessDiagnostic {
                level: DiagnosticLevel::Warning,
                kind: DiagnosticKind::SubscriberAlreadyInstalled,
                os_code: None,
                count: None,
                fallback: Some(DiagnosticFallback::ExistingSubscriber),
            });
            // Drop our created worker guard boundedly immediately
            drop(guard);
            drop(console);
            TracingGuard {
                file: None,
                console: None,
                sampler: None,
                active_mode: TracingMode::ExistingSubscriber,
                diagnostic: sink,
            }
        }
    }
}
