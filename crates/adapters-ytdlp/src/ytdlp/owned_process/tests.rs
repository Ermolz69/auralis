#![allow(clippy::unwrap_used)]

use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const CHILD: &str = "ytdlp::owned_process::tests::owned_process_child";

#[test]
fn owned_process_child() {
    let Ok(root) = std::env::var("AURALIS_PROCESS_TEST_ROOT") else {
        return;
    };
    let role = std::env::var("AURALIS_PROCESS_TEST_ROLE").unwrap();
    let root = Path::new(&root);
    if role == "success" {
        return;
    }
    if role == "leaf" {
        loop {
            std::fs::write(root.join("heartbeat"), format!("{:?}", Instant::now())).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    if role == "tree" {
        let mut child = child_command(root, "leaf").spawn().unwrap();
        child.wait().unwrap();
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut command = tokio::process::Command::from(child_command(root, "tree"));
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        super::spawn(command)
            .unwrap()
            .wait_with_output()
            .await
            .unwrap();
    });
}

fn child_command(root: &Path, role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", CHILD, "--nocapture"])
        .env("AURALIS_PROCESS_TEST_ROOT", root)
        .env("AURALIS_PROCESS_TEST_ROLE", role);
    command
}

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn a_successful_owned_process_returns_without_waiting_on_the_lifetime_monitor() {
    let root = tempfile::tempdir().unwrap();
    let child = super::spawn(tokio::process::Command::from(child_command(
        root.path(),
        "success",
    )))
    .unwrap();
    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(output.status.success());
}

#[cfg(unix)]
#[tokio::test]
async fn missing_download_candidates_still_return_missing_ytdlp() {
    let root = tempfile::tempdir().unwrap();
    let result = crate::ytdlp::command::run_ytdlp_download(
        &[root.path().join("missing")],
        "https://youtube.com/watch?v=test",
        root.path(),
        "original.%(ext)s",
        5000,
    )
    .await;
    assert!(matches!(
        result,
        Err(crate::ytdlp::error::YtDlpError::MissingYtDlp)
    ));
}

#[test]
fn killing_the_owner_stops_the_downloader_and_its_grandchildren() {
    let root = tempfile::tempdir().unwrap();
    let mut owner = KillOnDrop(
        child_command(root.path(), "owner")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let started = Instant::now();
    let heartbeat = root.path().join("heartbeat");
    while !heartbeat.exists() {
        assert!(owner.0.try_wait().unwrap().is_none());
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(10));
    }
    owner.0.kill().unwrap();
    owner.0.wait().unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let last = std::fs::read(&heartbeat).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        std::fs::read(heartbeat).unwrap(),
        last,
        "orphaned downloader still writes after owner death"
    );
}
