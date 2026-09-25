use process_wrap::tokio::{ChildWrapper, CommandWrap, CreationFlags, JobObject, KillOnDrop};
use std::io;
use tokio::process::Command;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

pub struct OwnedChild(Box<dyn ChildWrapper>);

impl OwnedChild {
    pub fn exited(&mut self) -> io::Result<bool> {
        Ok(self.0.try_wait()?.is_some())
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
