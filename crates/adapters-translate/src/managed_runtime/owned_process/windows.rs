use process_wrap::tokio::{ChildWrapper, CommandWrap, CreationFlags, JobObject, KillOnDrop};
use std::io;
use tokio::process::Command;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

pub struct OwnedChild(Box<dyn ChildWrapper>);

impl OwnedChild {
    pub fn exited(&mut self) -> io::Result<bool> {
        Ok(self.0.try_wait()?.is_some())
    }

    pub async fn stop(&mut self) -> io::Result<()> {
        if self.0.try_wait()?.is_none() {
            Box::into_pin(self.0.kill()).await?;
        }
        self.0.wait().await?;
        Ok(())
    }
}

pub fn spawn(command: Command) -> io::Result<OwnedChild> {
    let child = CommandWrap::from(command)
        .wrap(CreationFlags(CREATE_NO_WINDOW))
        .wrap(KillOnDrop)
        .wrap(JobObject)
        .spawn()?;
    Ok(OwnedChild(child))
}
