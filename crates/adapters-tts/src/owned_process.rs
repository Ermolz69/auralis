use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use std::io;
use std::time::Duration;
use tokio::process::Command;

const EXIT_DEADLINE: Duration = Duration::from_secs(5);

#[cfg(windows)]
use process_wrap::tokio::{CreationFlags, JobObject};
#[cfg(windows)]
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

#[cfg(not(windows))]
use process_wrap::tokio::ProcessGroup;

pub fn spawn(command: Command) -> io::Result<Box<dyn ChildWrapper>> {
    #[cfg(windows)]
    {
        CommandWrap::from(command)
            .wrap(CreationFlags(CREATE_NO_WINDOW))
            .wrap(KillOnDrop)
            .wrap(JobObject)
            .spawn()
    }
    #[cfg(not(windows))]
    {
        CommandWrap::from(command)
            .wrap(KillOnDrop)
            .wrap(ProcessGroup)
            .spawn()
    }
}

pub(crate) async fn terminate(
    child: &mut Box<dyn ChildWrapper>,
    resource: &str,
) -> Result<(), ports::PortError> {
    let deadline = tokio::time::Instant::now() + EXIT_DEADLINE;
    let _ = child.start_kill();
    tokio::time::timeout_at(deadline, child.wait())
        .await
        .map_err(|_| ports::PortError::CleanupFailed {
            resource: resource.into(),
            message: "Process tree termination could not be confirmed within 5 seconds".into(),
        })?
        .map_err(|error| ports::PortError::CleanupFailed {
            resource: resource.into(),
            message: format!("Process tree exit could not be confirmed: {error}"),
        })?;
    Ok(())
}

#[cfg(test)]
mod tests;
