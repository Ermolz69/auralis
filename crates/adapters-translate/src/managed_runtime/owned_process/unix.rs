use std::io;
use tokio::process::{Child, Command};

pub struct OwnedChild(Child);

impl OwnedChild {
    pub fn exited(&mut self) -> io::Result<bool> {
        Ok(self.0.try_wait()?.is_some())
    }
}

pub fn spawn(mut command: Command) -> io::Result<OwnedChild> {
    command.kill_on_drop(true);
    Ok(OwnedChild(command.spawn()?))
}
