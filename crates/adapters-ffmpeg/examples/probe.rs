use adapters_ffmpeg::ffprobe::FfprobeAdapter;
use ports::media::MediaProbePort;
use std::env;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -p adapters-ffmpeg --example probe -- <file_path>");
        std::process::exit(1);
    }

    let file_path = Path::new(&args[1]);

    println!("Probing file: {:?}", file_path);

    let candidates = vec![
        PathBuf::from("../../src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe"),
        PathBuf::from("../../src-tauri/binaries/ffprobe-aarch64-apple-darwin"),
        PathBuf::from("ffprobe"),
    ];

    let adapter = FfprobeAdapter::new(candidates);

    match adapter.probe_local_file(file_path).await {
        Ok(metadata) => {
            println!("Probe successful!\n{:#?}", metadata);
        }
        Err(e) => {
            eprintln!("Probe failed: {}", e);
            std::process::exit(1);
        }
    }
}
