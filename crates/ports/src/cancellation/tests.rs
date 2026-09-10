#![allow(clippy::unwrap_used)]

use super::CancelHandle;
use std::time::Duration;

#[tokio::test]
async fn cancellation_before_wait_is_observed() {
    let (handle, token) = CancelHandle::new();
    handle.cancel();

    tokio::time::timeout(Duration::from_millis(100), token.cancelled())
        .await
        .unwrap();
}

#[tokio::test]
async fn cancellation_wakes_every_waiter_and_is_idempotent() {
    let (handle, token) = CancelHandle::new();
    let mut waiters = Vec::new();
    for _ in 0..256 {
        let token = token.clone();
        waiters.push(tokio::spawn(async move {
            token.cancelled().await;
        }));
    }

    handle.cancel();
    handle.cancel();

    for waiter in waiters {
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .unwrap()
            .unwrap();
    }
    assert!(handle.is_cancelled());
    assert!(token.is_cancelled());
}
