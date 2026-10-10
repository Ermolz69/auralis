#![allow(clippy::unwrap_used)]

use std::{io::Write, process::Stdio, time::Duration};
use tokio::io::{AsyncBufReadExt, BufReader};

const CHILD: &str = "ytdlp::owned_process::queue_tests::queue_process_child";

#[test]
fn queue_process_child() {
    let Ok(role) = std::env::var("AURALIS_QUEUE_PROCESS_ROLE") else {
        return;
    };
    if role == "leaf" {
        println!("QUEUE_CHILD_READY");
        std::io::stdout().flush().unwrap();
        let (_send, receive) = std::sync::mpsc::channel::<()>();
        receive.recv().unwrap();
        return;
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--nocapture"])
        .env("AURALIS_QUEUE_PROCESS_ROLE", "leaf")
        .spawn()
        .unwrap();
    child.wait().unwrap();
}

#[tokio::test]
async fn queue_process_cancel_waits_for_tree_exit_and_drains_pipes() {
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", CHILD, "--nocapture"])
        .env("AURALIS_QUEUE_PROCESS_ROLE", "tree")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = super::spawn(command).unwrap();
    let mut stdout = BufReader::new(child.stdout().take().unwrap());
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let mut line = String::new();
            assert!(stdout.read_line(&mut line).await.unwrap() > 0);
            if line.contains("QUEUE_CHILD_READY") {
                break;
            }
        }
    })
    .await
    .unwrap();
    *child.stdout() = Some(stdout.into_inner());
    let token = ports::cancellation::CancellationToken::new();
    token.cancel();
    let result =
        super::cancellable::wait(child, &token, super::cancellable::ProcessLimits::default()).await;
    assert!(matches!(result, Err(ports::PortError::Cancelled)));
}
