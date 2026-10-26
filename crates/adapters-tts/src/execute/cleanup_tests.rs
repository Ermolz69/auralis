use super::*;
use std::{
    future::Future,
    io,
    pin::Pin,
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug)]
struct UnconfirmedChild {
    kill_calls: Arc<AtomicUsize>,
    wait_calls: Arc<AtomicUsize>,
    wait_fails: bool,
}

impl ChildWrapper for UnconfirmedChild {
    fn inner(&self) -> &dyn ChildWrapper {
        self
    }

    fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
        self
    }

    fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
        self
    }

    fn start_kill(&mut self) -> io::Result<()> {
        self.kill_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn wait(&mut self) -> Pin<Box<dyn Future<Output = io::Result<ExitStatus>> + Send + '_>> {
        self.wait_calls.fetch_add(1, Ordering::SeqCst);
        if self.wait_fails {
            Box::pin(async { Err(io::Error::other("fixture exit inspection failed")) })
        } else {
            Box::pin(std::future::pending())
        }
    }
}

async fn check_cleanup(wait_fails: bool) -> Result<PortError, Box<dyn std::error::Error>> {
    let kill_calls = Arc::new(AtomicUsize::new(0));
    let wait_calls = Arc::new(AtomicUsize::new(0));
    let mut child: Box<dyn ChildWrapper> = Box::new(UnconfirmedChild {
        kill_calls: kill_calls.clone(),
        wait_calls: wait_calls.clone(),
        wait_fails,
    });
    let result = timeout(
        EXIT_DEADLINE + Duration::from_secs(2),
        terminate(&mut child),
    )
    .await
    .map_err(|_| "Forced cleanup waited outside its exit deadline")?;
    assert_eq!(kill_calls.load(Ordering::SeqCst), 1);
    assert_eq!(wait_calls.load(Ordering::SeqCst), 1);
    result
        .err()
        .ok_or_else(|| "Unconfirmed exit was accepted".into())
}

#[tokio::test]
async fn pending_process_exit_cannot_block_the_cleanup_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    let error = check_cleanup(false).await?;
    assert!(matches!(error, PortError::CleanupFailed { message, .. }
        if message.contains("termination could not be confirmed")
            && message.contains("model admission retained")));
    Ok(())
}

#[tokio::test]
async fn failed_exit_inspection_retains_unconfirmed_cleanup()
-> Result<(), Box<dyn std::error::Error>> {
    let error = check_cleanup(true).await?;
    assert!(matches!(error, PortError::CleanupFailed { message, .. }
        if message.contains("fixture exit inspection failed")
            && message.contains("model admission retained")));
    Ok(())
}
