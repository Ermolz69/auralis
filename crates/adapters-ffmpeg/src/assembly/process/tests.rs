use super::*;
use std::{fs, io::Write, path::Path, time::Instant};

fn child(mode: &str, marker: &Path) -> Command {
    let executable = match std::env::current_exe() {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let mut command = Command::new(executable);
    command
        .args([
            "--exact",
            "assembly::process::tests::fixture",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("AURALIS_AUDIO_CHILD", mode)
        .env("AURALIS_AUDIO_CHILD_MARKER", marker);
    command
}

#[test]
fn fixture() {
    let Ok(mode) = std::env::var("AURALIS_AUDIO_CHILD") else {
        return;
    };
    if let Some(marker) = std::env::var_os("AURALIS_AUDIO_CHILD_MARKER") {
        assert!(fs::write(marker, std::process::id().to_string()).is_ok());
    }
    match mode.as_str() {
        "error" => std::process::exit(17),
        "flood" => {
            let _ = std::io::stderr().write_all(&vec![b'x'; 512 * 1024]);
        }
        _ => {}
    }
    std::thread::sleep(Duration::from_secs(30));
}

fn temporary() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/revoice/assembly-unit");
    assert!(fs::create_dir_all(&root).is_ok());
    match tempfile::tempdir_in(root) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

#[tokio::test]
async fn cancellation_after_verified_child_start_waits_for_owned_exit() {
    let directory = temporary();
    let marker = directory.path().join("started");
    let token = CancellationToken::new();
    let trigger = token.clone();
    let watched = marker.clone();
    let watcher = tokio::spawn(async move {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !watched.exists() && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let started = watched.exists();
        trigger.cancel();
        started
    });
    let result = run(child("hang", &marker), token, Duration::from_secs(10), 1024).await;
    assert!(matches!(watcher.await, Ok(true)));
    assert!(matches!(result, Err(PortError::Cancelled)));
}

#[tokio::test]
async fn timeout_and_stderr_limits_reap_the_controlled_child() {
    let directory = temporary();
    let marker = directory.path().join("started");
    let started = Instant::now();
    let result = run(
        child("hang", &marker),
        CancellationToken::new(),
        Duration::from_millis(50),
        1024,
    )
    .await;
    assert!(
        matches!(result, Err(PortError::ExternalToolFailed { message, .. }) if message.contains("deadline"))
    );
    assert!(started.elapsed() < Duration::from_secs(6));
    let result = run(
        child("flood", &marker),
        CancellationToken::new(),
        Duration::from_secs(10),
        1024,
    )
    .await;
    assert!(
        matches!(result, Err(PortError::ExternalToolFailed { message, .. }) if message.contains("stderr byte limit"))
    );
}

#[tokio::test]
async fn nonzero_status_is_not_success() {
    let directory = temporary();
    let result = run(
        child("error", &directory.path().join("started")),
        CancellationToken::new(),
        Duration::from_secs(10),
        1024,
    )
    .await;
    assert!(
        matches!(result, Err(PortError::ExternalToolFailed { message, .. }) if message.contains("17"))
    );
}
