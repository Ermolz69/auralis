#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(e) = auralis_app::run() {
        use auralis_app::observability::diagnostic::{DiagnosticSink, StderrDiagnosticSink};
        let (sink, owner) = StderrDiagnosticSink::owned();
        sink.emit(e.diagnostic());
        owner.shutdown(auralis_app::TRACING_FLUSH_TIMEOUT);
        std::process::exit(1);
    }
}
