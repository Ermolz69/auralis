#![cfg(windows)]

use common::machine_admission::MachineAdmission;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn admission(path: PathBuf, name: String) -> Result<MachineAdmission, io::Error> {
    MachineAdmission::new(path, name, Arc::new(|_| Ok(())))
}

fn unique_name() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    format!("Local\\AuralisContractTest-{}-{nonce}", std::process::id())
}

#[tokio::test]
async fn duplicate_admission_rejects_then_releases_after_exit() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let gate = admission(root.path().join("model-server.lock"), unique_name())?;
    let held = gate.acquire().await?;
    let duplicate = match gate.acquire().await {
        Ok(_) => return Err("duplicate admission accepted".into()),
        Err(error) => error,
    };
    assert_eq!(duplicate.kind(), io::ErrorKind::WouldBlock);
    held.release().await?;
    let recovered = gate.acquire().await?;
    recovered.release().await?;
    Ok(())
}

#[tokio::test]
async fn uncertain_external_process_blocks_without_leaking_admission() -> Result<(), Box<dyn Error>>
{
    let root = tempfile::tempdir()?;
    let path = root.path().join("model-server.lock");
    let name = unique_name();
    let blocked = MachineAdmission::new(
        path.clone(),
        name.clone(),
        Arc::new(|_| {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "synthetic external model server",
            ))
        }),
    )?;
    let external = match blocked.acquire().await {
        Ok(_) => return Err("external server accepted".into()),
        Err(error) => error,
    };
    assert_eq!(external.kind(), io::ErrorKind::WouldBlock);
    admission(path, name)?.acquire().await?.release().await?;
    Ok(())
}

#[tokio::test]
async fn dropped_pending_admission_releases_after_inspection_finishes() -> Result<(), Box<dyn Error>>
{
    let root = tempfile::tempdir()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let gate = MachineAdmission::new(
        root.path().join("model-server.lock"),
        unique_name(),
        Arc::new(move |_| {
            if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                std::thread::sleep(Duration::from_millis(150));
            }
            Ok(())
        }),
    )?;
    assert!(
        tokio::time::timeout(Duration::from_millis(20), gate.acquire())
            .await
            .is_err()
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    gate.acquire().await?.release().await?;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    Ok(())
}

#[test]
fn process_admission_helper() -> Result<(), Box<dyn Error>> {
    let Ok(lock_path) = std::env::var("AURALIS_ADMISSION_CHILD_LOCK") else {
        return Ok(());
    };
    let name = std::env::var("AURALIS_ADMISSION_CHILD_MUTEX")?;
    let ready = PathBuf::from(std::env::var("AURALIS_ADMISSION_CHILD_READY")?);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let lease = runtime.block_on(admission(PathBuf::from(lock_path), name)?.acquire())?;
    std::fs::write(ready, b"ready")?;
    std::thread::sleep(Duration::from_secs(30));
    runtime.block_on(lease.release())?;
    Ok(())
}

#[tokio::test]
async fn killed_owner_releases_mutex_and_file_lock() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let lock = root.path().join("model-server.lock");
    let ready = root.path().join("ready");
    let name = unique_name();
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", "process_admission_helper", "--nocapture"])
        .env("AURALIS_ADMISSION_CHILD_LOCK", &lock)
        .env("AURALIS_ADMISSION_CHILD_MUTEX", &name)
        .env("AURALIS_ADMISSION_CHILD_READY", &ready)
        .spawn()?;
    let started = std::time::Instant::now();
    while !Path::new(&ready).exists() {
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("synthetic owner did not acquire admission".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let gate = admission(lock, name)?;
    let occupied = match gate.acquire().await {
        Ok(_) => return Err("live owner accepted".into()),
        Err(error) => error,
    };
    assert_eq!(occupied.kind(), io::ErrorKind::WouldBlock);
    child.kill()?;
    child.wait()?;
    gate.acquire().await?.release().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires an explicitly installed model-free Python with portalocker"]
async fn portalocker_interoperability_rejects_then_recovers() -> Result<(), Box<dyn Error>> {
    let python = std::env::var("AURALIS_CONTRACT_PYTHON")?;
    let root = tempfile::tempdir()?;
    let lock = root.path().join("model-server.lock");
    let ready = root.path().join("ready");
    let helper =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/portalocker_owner.py");
    let mut child = std::process::Command::new(python)
        .args([helper.as_os_str(), lock.as_os_str(), ready.as_os_str()])
        .spawn()?;
    let result = async {
        let started = std::time::Instant::now();
        while !ready.exists() {
            if child.try_wait()?.is_some() || started.elapsed() > Duration::from_secs(5) {
                return Err::<(), Box<dyn Error>>("portalocker owner did not acquire lock".into());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let gate = admission(lock.clone(), unique_name())?;
        let occupied = match gate.acquire().await {
            Ok(_) => return Err("portalocker owner was not detected".into()),
            Err(error) => error,
        };
        assert_eq!(occupied.kind(), io::ErrorKind::WouldBlock);
        Ok(())
    }
    .await;
    let _ = child.kill();
    child.wait()?;
    result?;
    admission(lock, unique_name())?
        .acquire()
        .await?
        .release()
        .await?;
    Ok(())
}
