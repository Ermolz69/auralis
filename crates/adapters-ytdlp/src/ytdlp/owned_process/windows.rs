use process_wrap::tokio::{ChildWrapper, CommandWrap, CreationFlags, JobObject, KillOnDrop};
use tokio::process::Command;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;

pub struct OwnedChild(Box<dyn ChildWrapper>);

impl OwnedChild {
    pub async fn wait_with_output(self) -> std::io::Result<std::process::Output> {
        std::pin::Pin::from(self.0.wait_with_output()).await
    }
}

pub fn spawn(command: Command) -> std::io::Result<OwnedChild> {
    let child = CommandWrap::from(command)
        .wrap(CreationFlags(CREATE_NO_WINDOW))
        .wrap(KillOnDrop)
        .wrap(JobObject)
        .spawn()?;
    Ok(OwnedChild(child))
}
