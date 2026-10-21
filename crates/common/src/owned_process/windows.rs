use process_wrap::tokio::{ChildWrapper, CommandWrap, CreationFlags, JobObject, KillOnDrop};
use std::io;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout, Command};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

pub struct OwnedChild(Box<dyn ChildWrapper>);

impl OwnedChild {
    pub fn id(&self) -> Option<u32> {
        self.0.id()
    }

    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.0.stdin().take()
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.0.stdout().take()
    }

    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.0.stderr().take()
    }

    pub fn exited(&mut self) -> io::Result<bool> {
        Ok(self.0.try_wait()?.is_some())
    }

    pub async fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        self.0.wait().await
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
