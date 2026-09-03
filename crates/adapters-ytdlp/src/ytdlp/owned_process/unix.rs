use std::{
    io,
    process::{Output, Stdio},
};
use tokio::process::{Child, ChildStdin, Command};

pub struct OwnedChild {
    child: Child,
    lifetime: ChildStdin,
}

impl OwnedChild {
    pub async fn wait_with_output(self) -> io::Result<Output> {
        let Self { child, lifetime } = self;
        let result = child.wait_with_output().await;
        drop(lifetime);
        result
    }
}

pub fn spawn(command: Command) -> io::Result<OwnedChild> {
    let original = command.as_std();
    // The monitor survives SIGKILL of the app. EOF on the app-owned pipe kills
    // the entire private process group, including yt-dlp's ffmpeg children.
    let mut wrapper = Command::new("/bin/sh");
    wrapper.args(["-c", "group=$$; exec 3<&0; (read -r token; kill -KILL -\"$group\") <&3 >/dev/null 2>&1 & \"$@\" 3<&- </dev/null", "auralis-download"])
        .arg(original.get_program()).args(original.get_args())
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .process_group(0).kill_on_drop(true);
    for (key, value) in original.get_envs() {
        if let Some(value) = value {
            wrapper.env(key, value);
        } else {
            wrapper.env_remove(key);
        }
    }
    if let Some(dir) = original.get_current_dir() {
        wrapper.current_dir(dir);
    }
    let mut child = wrapper.spawn()?;
    let lifetime = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("Missing process lifetime pipe"))?;
    Ok(OwnedChild { child, lifetime })
}
