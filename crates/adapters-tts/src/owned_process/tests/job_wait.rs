use super::super::*;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use tokio::io::{AsyncBufReadExt, BufReader};
use windows::Win32::{
    Foundation::{FILETIME, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        WaitForSingleObject,
    },
};

struct Process {
    handle: OwnedHandle,
    pid: u32,
    creation_time: u64,
}

fn creation_time(handle: &OwnedHandle) -> io::Result<u64> {
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    unsafe {
        GetProcessTimes(
            HANDLE(handle.as_raw_handle()),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    }
    .map_err(io::Error::other)?;
    Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}

impl Process {
    fn from_handle(pid: u32, handle: OwnedHandle) -> io::Result<Self> {
        let creation_time = creation_time(&handle)?;
        Ok(Self {
            handle,
            pid,
            creation_time,
        })
    }

    fn open(pid: u32) -> io::Result<Self> {
        // The PID comes from the owned parent; access only waits and queries its identity.
        let handle = unsafe {
            OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                pid,
            )
        }
        .map_err(io::Error::other)?;
        Self::from_handle(pid, unsafe { OwnedHandle::from_raw_handle(handle.0) })
    }

    fn exited(&self) -> io::Result<bool> {
        // The owned handle stays open for this nonblocking query.
        match unsafe { WaitForSingleObject(HANDLE(self.handle.as_raw_handle()), 0) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(io::Error::last_os_error()),
        }
    }

    async fn wait(&self) -> io::Result<()> {
        tokio::time::timeout(EXIT_DEADLINE, async {
            while !self.exited()? {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            if creation_time(&self.handle)? != self.creation_time {
                return Err(io::Error::other("fixture process identity changed"));
            }
            eprintln!(
                "confirmed fixture exit: pid={} creation_time={}",
                self.pid, self.creation_time
            );
            Ok(())
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "fixture process is still alive"))?
    }
}

async fn fixture(mode: &str) -> io::Result<(Box<dyn ChildWrapper>, Process, Process)> {
    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("job_wait.ps1"),
        ])
        .env("AURALIS_TEST_JOB_MODE", mode)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = spawn(command)?;
    let parent = Process::from_handle(
        child
            .id()
            .ok_or_else(|| io::Error::other("fixture parent PID missing"))?,
        child
            .process_handle()
            .ok_or_else(|| io::Error::other("fixture parent handle missing"))?
            .try_clone_to_owned()?,
    )?;
    let stdout = child
        .stdout()
        .take()
        .ok_or_else(|| io::Error::other("fixture stdout missing"))?;
    let mut reader = BufReader::new(stdout);
    let mut pid = String::new();
    tokio::time::timeout(EXIT_DEADLINE, reader.read_line(&mut pid))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "fixture child PID missing"))??;
    let descendant = Process::open(pid.trim().parse().map_err(io::Error::other)?)?;
    parent.wait().await?;
    Ok((child, parent, descendant))
}

async fn exercise(active_worker: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (mut child, parent, descendant) = fixture("linger").await?;
    // Drain queued start/parent-exit packets on the upstream implementation. They
    // must not make a later cancelled wait report completion for a live child.
    for _ in 0..128 {
        let _ = child.try_wait()?;
    }
    let tried = child.try_wait()?;
    let alive_before = !descendant.exited()?;
    let prior = tokio::time::timeout(Duration::from_millis(100), child.wait()).await;
    let repeated = tokio::time::timeout(Duration::from_millis(100), child.wait()).await;
    let alive_after = !descendant.exited()?;
    let cleanup = if active_worker {
        crate::execute::terminate(&mut child).await
    } else {
        terminate(&mut child, "fixture component").await
    };
    let exited_when_cleanup_returned = descendant.exited()?;
    let native_exit = descendant.wait().await;
    cleanup?;
    native_exit?;
    parent.wait().await?;
    assert!(
        exited_when_cleanup_returned,
        "cleanup returned before native descendant exit"
    );
    assert!(
        alive_before && alive_after,
        "fixture descendant did not linger"
    );
    assert!(prior.is_err(), "prior wait accepted parent exit: {prior:?}");
    assert!(
        repeated.is_err(),
        "cancelled wait cached parent exit: {repeated:?}"
    );
    assert!(tried.is_none(), "try_wait accepted parent exit: {tried:?}");
    Ok(())
}

#[tokio::test]
async fn cancelled_component_wait_keeps_the_lingering_descendant_in_scope()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(25), exercise(false)).await?
}

#[tokio::test]
async fn active_synthesis_cleanup_confirms_descendant_exit_after_a_cancelled_wait()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(25), exercise(true)).await?
}

async fn exercise_normal(mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (mut child, parent, descendant) = fixture(mode).await?;
    let result = tokio::time::timeout(EXIT_DEADLINE, child.wait()).await;
    let exited_at_return = descendant.exited()?;
    let cleanup = terminate(&mut child, "normally completed fixture").await;
    let native_exit = descendant.wait().await;
    cleanup?;
    native_exit?;
    parent.wait().await?;
    assert!(result??.success());
    assert!(
        exited_at_return,
        "normal wait returned before native descendant exit"
    );
    Ok(())
}

#[tokio::test]
async fn normal_wait_confirms_short_lived_descendants() -> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(25), exercise_normal("short")).await?
}

#[tokio::test]
async fn normal_wait_accepts_descendants_completed_before_observation()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(25), exercise_normal("finished")).await?
}
