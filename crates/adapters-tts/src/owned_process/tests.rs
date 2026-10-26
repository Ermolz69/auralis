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
async fn component_cleanup_deadline_covers_pending_exit_without_a_real_process()
-> Result<(), Box<dyn std::error::Error>> {
    let mut child: Box<dyn ChildWrapper> = Box::new(UnconfirmedChild { wait_fails: false });
    let result = tokio::time::timeout(
        EXIT_DEADLINE + Duration::from_secs(2),
        terminate(&mut child, "fixture component"),
    )
    .await?;
    assert!(
        matches!(result, Err(ports::PortError::CleanupFailed { resource, message })
        if resource == "fixture component" && message.contains("within 5 seconds"))
    );
    Ok(())
}

#[tokio::test]
async fn component_cleanup_retains_a_typed_unconfirmed_exit_error()
-> Result<(), Box<dyn std::error::Error>> {
    let mut child: Box<dyn ChildWrapper> = Box::new(UnconfirmedChild { wait_fails: true });
    let result = terminate(&mut child, "fixture component").await;
    assert!(
        matches!(result, Err(ports::PortError::CleanupFailed { resource, message })
        if resource == "fixture component" && message.contains("fixture exit inspection failed"))
    );
    Ok(())
}
