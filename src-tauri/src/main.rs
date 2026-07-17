#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(e) = auralis_app::run() {
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), "Auralis failed to start: {}", e);
        std::process::exit(1);
    }
}
