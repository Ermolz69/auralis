use std::io;
use std::time::Duration;
use tokio::process::{Child, Command};

const EXIT_DEADLINE: Duration = Duration::from_secs(5);

pub struct OwnedChild(Child);

impl OwnedChild {
    pub fn exited(&mut self) -> io::Result<bool> {
        Ok(self.0.try_wait()?.is_some())
    }

    pub async fn stop(&mut self) -> io::Result<()> {
        let deadline = tokio::time::Instant::now() + EXIT_DEADLINE;
        let _ = self.0.start_kill();
        tokio::time::timeout_at(deadline, self.0.wait())
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

pub fn spawn(mut command: Command) -> io::Result<OwnedChild> {
    command.kill_on_drop(true);
    Ok(OwnedChild(command.spawn()?))
}
