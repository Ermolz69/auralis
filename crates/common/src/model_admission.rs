use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

pub struct ModelAdmissionLease {
    child: Child,
}

impl ModelAdmissionLease {
    pub fn acquire() -> io::Result<Self> {
        Self::acquire_mode(false)
    }
    pub fn acquire_model_free() -> io::Result<Self> {
        Self::acquire_mode(true)
    }
    pub fn guardian_pid(&self) -> u32 {
        self.child.id()
    }
    pub fn release(mut self) -> io::Result<()> {
        if let Some(mut stdin) = self.child.stdin.take() {
            let _ = stdin.write_all(b"release\n");
        }
        for _ in 0..1000 {
            match self.child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(_)) => return Err(io::Error::other("Admission release inspection failed")),
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                Err(error) => {
                    std::mem::forget(self);
                    return Err(error);
                }
            }
        }
        // Keep the guardian and its leases alive when resource release is uncertain.
        std::mem::forget(self);
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Admission release was not confirmed",
        ))
    }
    fn acquire_mode(model_free: bool) -> io::Result<Self> {
        if !cfg!(windows) {
            return Err(io::Error::other(
                "PUZA model admission requires Windows process inspection",
            ));
        }
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command"]);
        command.arg(if model_free {
            format!("& {{ {} }} -ModelFree", include_str!("model_admission.ps1"))
        } else {
            include_str!("model_admission.ps1").into()
        });
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Admission pipe missing"))?;
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = send.send(result);
        });
        match receive.recv_timeout(std::time::Duration::from_secs(20)) {
            Ok(Ok(line)) if line.trim() == "READY" => Ok(Self { child }),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                let reason = match result {
                    Ok(Ok(line)) => line.trim().to_string(),
                    _ => "Process/GPU inspection did not complete".into(),
                };
                Err(io::Error::other(reason))
            }
        }
    }
}

impl Drop for ModelAdmissionLease {
    fn drop(&mut self) {
        if let Some(mut stdin) = self.child.stdin.take() {
            let _ = stdin.write_all(b"release\n");
        }
        for _ in 0..1000 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // The admission helper retains both leases until resource release is established.
    }
}
