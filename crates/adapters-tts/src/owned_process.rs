use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use std::io;
use tokio::process::Command;

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
