use std::fs::{File, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use tokio::sync::oneshot;

pub type InspectExternal = Arc<dyn Fn(u32) -> io::Result<()> + Send + Sync>;

#[derive(Clone)]
pub struct MachineAdmission {
    lock_path: PathBuf,
    mutex_name: String,
    inspect_external: InspectExternal,
}

pub struct ModelAdmissionLease {
    release: Option<mpsc::Sender<()>>,
    owner: Option<JoinHandle<()>>,
}

impl MachineAdmission {
    pub fn new(
        lock_path: PathBuf,
        mutex_name: String,
        inspect_external: InspectExternal,
    ) -> io::Result<Self> {
        if !lock_path.is_absolute() || mutex_name.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "admission paths and mutex name must be explicit",
            ));
        }
        Ok(Self {
            lock_path,
            mutex_name,
            inspect_external,
        })
    }

    pub fn system() -> io::Result<Self> {
        let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is unavailable")
        })?;
        Self::new(
            PathBuf::from(local).join("PUZA/model-server.lock"),
            "Global\\PUZA_TTS_NATIVE_RENDER".into(),
            Arc::new(inspect_system_processes),
        )
    }

    pub async fn acquire(&self) -> io::Result<ModelAdmissionLease> {
        let (ready_tx, ready_rx) = oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let lock_path = self.lock_path.clone();
        let mutex_name = self.mutex_name.clone();
        let inspect = self.inspect_external.clone();
        let owner = std::thread::spawn(move || {
            let admitted = acquire_owned(&lock_path, &mutex_name, &inspect);
            let ok = admitted.is_ok();
            if ready_tx
                .send(
                    admitted
                        .as_ref()
                        .map(|_| ())
                        .map_err(|error| io::Error::new(error.kind(), error.to_string())),
                )
                .is_err()
            {
                return;
            }
            if ok {
                let _ = release_rx.recv();
            }
            drop(admitted);
        });
        match ready_rx.await {
            Ok(Ok(())) => Ok(ModelAdmissionLease {
                release: Some(release_tx),
                owner: Some(owner),
            }),
            Ok(Err(error)) => {
                let _ = owner.join();
                Err(error)
            }
            Err(_) => {
                let _ = owner.join();
                Err(io::Error::other(
                    "model admission owner exited before reporting",
                ))
            }
        }
    }
}

impl ModelAdmissionLease {
    pub async fn release(mut self) -> io::Result<()> {
        self.signal();
        if let Some(owner) = self.owner.take() {
            tokio::task::spawn_blocking(move || owner.join())
                .await
                .map_err(|error| io::Error::other(error.to_string()))?
                .map_err(|_| io::Error::other("model admission owner panicked"))?;
        }
        Ok(())
    }

    fn signal(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

impl Drop for ModelAdmissionLease {
    fn drop(&mut self) {
        self.signal();
    }
}

#[cfg(windows)]
struct HeldAdmission {
    _file: File,
    _mutex: WindowsMutex,
}

#[cfg(windows)]
struct WindowsMutex(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for WindowsMutex {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::System::Threading::ReleaseMutex(self.0);
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
fn acquire_owned(
    lock_path: &PathBuf,
    mutex_name: &str,
    inspect: &InspectExternal,
) -> io::Result<HeldAdmission> {
    use windows::Win32::Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
    let name = windows::core::HSTRING::from(mutex_name);
    let handle = unsafe { CreateMutexW(None, false, &name) }
        .map_err(|error| io::Error::other(error.to_string()))?;
    let wait = unsafe { WaitForSingleObject(handle, 0) };
    if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
        unsafe {
            let _ = CloseHandle(handle);
        }
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "another native model operation owns admission",
        ));
    }
    let mutex = WindowsMutex(handle);
    let parent = lock_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "admission lock has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)?;
    file.try_lock()?;
    inspect(std::process::id())?;
    Ok(HeldAdmission {
        _file: file,
        _mutex: mutex,
    })
}

#[cfg(not(windows))]
fn acquire_owned(
    _lock_path: &PathBuf,
    _mutex_name: &str,
    _inspect: &InspectExternal,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native model admission requires Windows",
    ))
}

#[cfg(windows)]
fn inspect_system_processes(own_pid: u32) -> io::Result<()> {
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "SystemRoot is unavailable"))?;
    let tasklist = PathBuf::from(system_root).join("System32/tasklist.exe");
    let output = std::process::Command::new(tasklist)
        .args(["/fo", "csv", "/nh"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other("external model process inspection failed"));
    }
    let listing = String::from_utf8_lossy(&output.stdout);
    for line in listing.lines() {
        let fields: Vec<&str> = line.trim_matches('"').split("\",\"").collect();
        if fields.len() < 2 {
            return Err(io::Error::other("external process listing is malformed"));
        }
        let pid: u32 = fields[1]
            .replace(',', "")
            .parse()
            .map_err(|_| io::Error::other("external process identity is malformed"))?;
        if pid == own_pid {
            continue;
        }
        let name = fields[0].to_ascii_lowercase();
        let executable = name.strip_suffix(".exe").unwrap_or(&name);
        if executable == "python"
            || executable == "pythonw"
            || executable.starts_with("python3")
            || executable.starts_with("pypy")
            || matches!(
                executable,
                "llama-server"
                    | "llama-cli"
                    | "ollama"
                    | "vllm"
                    | "text-marker"
                    | "tts"
                    | "tts-server"
            )
            || executable.contains("docker")
            || executable.contains("wsl")
        {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                format!("external model-capable process is present: {name} PID {pid}"),
            ));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn inspect_system_processes(_own_pid: u32) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "native model inspection requires Windows",
    ))
}
