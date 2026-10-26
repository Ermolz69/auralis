use process_wrap::tokio::{ChildWrapper, CommandWrap, CreationFlags, JobObject, KillOnDrop};
use std::io;
use std::time::Duration;
use tokio::process::Command;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

const EXIT_DEADLINE: Duration = Duration::from_secs(5);

pub struct OwnedChild(Box<dyn ChildWrapper>);

impl OwnedChild {
    pub fn exited(&mut self) -> io::Result<bool> {
        Ok(self.0.try_wait()?.is_some())
    }

    pub async fn stop(&mut self) -> io::Result<()> {
        let _ = self.0.start_kill();
        tokio::time::timeout(EXIT_DEADLINE, self.0.wait())
            .await
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Managed translation process tree exit was not confirmed within 5 seconds",
                )
            })??;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

pub fn spawn(command: Command) -> io::Result<OwnedChild> {
    let child = CommandWrap::from(command)
        .wrap(CreationFlags(CREATE_NO_WINDOW))
        .wrap(KillOnDrop)
        .wrap(JobObject)
        .spawn()?;
    Ok(OwnedChild(child))
}
