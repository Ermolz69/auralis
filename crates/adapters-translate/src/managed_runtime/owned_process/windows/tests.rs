use super::*;
use std::{future::Future, pin::Pin, process::ExitStatus};

#[derive(Debug)]
struct UnconfirmedChild {
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
        Ok(())
    }

    fn wait(&mut self) -> Pin<Box<dyn Future<Output = io::Result<ExitStatus>> + Send + '_>> {
        if self.wait_fails {
            Box::pin(async { Err(io::Error::other("fixture exit inspection failed")) })
        } else {
            Box::pin(std::future::pending())
        }
    }
}

#[tokio::test]
async fn managed_stop_bounds_a_pending_tree_wait_without_starting_a_server()
-> Result<(), Box<dyn std::error::Error>> {
    let mut child = OwnedChild(Box::new(UnconfirmedChild { wait_fails: false }));
    let result = tokio::time::timeout(EXIT_DEADLINE + Duration::from_secs(2), child.stop()).await?;
    let error = result.err().ok_or("Unconfirmed tree exit was accepted")?;
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("within 5 seconds"));
    Ok(())
}

#[tokio::test]
async fn managed_stop_preserves_exit_inspection_errors() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = OwnedChild(Box::new(UnconfirmedChild { wait_fails: true }));
    let error = child
        .stop()
        .await
        .err()
        .ok_or("Failed exit inspection was accepted")?;
    assert!(error.to_string().contains("fixture exit inspection failed"));
    Ok(())
}
