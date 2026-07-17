#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(e) = auralis_app::run() {
        use auralis_app::observability::diagnostic::{DiagnosticSink, StderrDiagnosticSink};
        StderrDiagnosticSink.emit(e.diagnostic());
        std::process::exit(1);
    }
}
