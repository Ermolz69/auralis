use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    InvalidRustLog,
    LogDirectoryUnavailable,
    FileAppenderUnavailable,
    SubscriberAlreadyInstalled,
    BufferOverflow,
    RuntimeLogWriteFailed,
    HealthSamplerUnavailable,
    TracingFlushTimedOut,
    ApplicationConfigurationInvalid,
    ObservabilityConfigurationInvalid,
    TauriBuildFailed,
    ShutdownFailed,
    ShutdownNotObserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFallback {
    ConsoleOnly,
    ExistingSubscriber,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessDiagnostic {
    pub level: DiagnosticLevel,
    pub kind: DiagnosticKind,
    pub os_code: Option<i32>,
    pub count: Option<u64>,
    pub fallback: Option<DiagnosticFallback>,
}

pub fn write_diagnostic(
    writer: &mut dyn Write,
    diagnostic: ProcessDiagnostic,
) -> std::io::Result<()> {
    let level_str = match diagnostic.level {
        DiagnosticLevel::Warning => "WARNING",
        DiagnosticLevel::Error => "ERROR",
    };
    let fallback_str = match diagnostic.fallback {
        Some(DiagnosticFallback::ConsoleOnly) => " fallback_mode=ConsoleOnly",
        Some(DiagnosticFallback::ExistingSubscriber) => " fallback_mode=ExistingSubscriber",
        None => "",
    };
    let code_str = match diagnostic.os_code {
        Some(c) => format!(" os_code={}", c),
        None => "".to_string(),
    };
    let count_str = match diagnostic.count {
        Some(c) => format!(" count={}", c),
        None => "".to_string(),
    };
    writeln!(
        writer,
        "{} [ProcessDiagnostic] kind={:?}{}{}{}",
        level_str, diagnostic.kind, code_str, count_str, fallback_str
    )
}

pub trait DiagnosticSink: Send + Sync {
    fn emit(&self, diag: ProcessDiagnostic);
}

pub(crate) struct TracingDiagnosticSink;
impl DiagnosticSink for TracingDiagnosticSink {
    fn emit(&self, diag: ProcessDiagnostic) {
        tracing::warn!(event_name = "process_diagnostic", kind = ?diag.kind, count = ?diag.count);
    }
}

pub struct StderrDiagnosticSink(pub(crate) tracing_appender::non_blocking::NonBlocking);

impl StderrDiagnosticSink {
    pub fn owned() -> (Self, super::sink::OwnedSink) {
        let owner = super::sink::OwnedSink::new(
            std::io::stderr(),
            super::bounded_writer::MAX_QUEUED_EVENTS,
            true,
        );
        (Self(owner.writer.clone()), owner)
    }
}

impl DiagnosticSink for StderrDiagnosticSink {
    fn emit(&self, diag: ProcessDiagnostic) {
        let mut record = Vec::new();
        if write_diagnostic(&mut record, diag).is_ok() {
            let _ = self.0.clone().write_all(&record);
        }
    }
}
