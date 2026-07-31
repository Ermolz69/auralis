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

pub fn stderr_writer() -> std::io::Stderr {
    std::io::stderr()
}

pub trait DiagnosticSink: Send + Sync {
    fn emit(&self, diag: ProcessDiagnostic);
}

pub struct StderrDiagnosticSink;

impl DiagnosticSink for StderrDiagnosticSink {
    fn emit(&self, diag: ProcessDiagnostic) {
        let mut stderr = stderr_writer().lock();
        let _ = write_diagnostic(&mut stderr, diag);
    }
}
