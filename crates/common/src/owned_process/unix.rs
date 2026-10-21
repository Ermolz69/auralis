use std::io;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

pub struct OwnedChild(Child);

impl OwnedChild {
    pub fn id(&self) -> Option<u32> {
        self.0.id()
    }

    pub fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.0.stdin.take()
    }

    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.0.stdout.take()
    }

    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.0.stderr.take()
    }

    pub fn exited(&mut self) -> io::Result<bool> {
        Ok(self.0.try_wait()?.is_some())
    }

    pub async fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        self.0.wait().await
    }

    pub async fn stop(&mut self) -> io::Result<()> {
        if self.0.try_wait()?.is_none() {
            self.0.kill().await?;
        }
        self.0.wait().await?;
        Ok(())
    }
}

pub fn spawn(mut command: Command) -> io::Result<OwnedChild> {
    command.kill_on_drop(true);
    Ok(OwnedChild(command.spawn()?))
}
