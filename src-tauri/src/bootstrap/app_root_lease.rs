use std::{
    fs::{File, OpenOptions},
    path::Path,
};

#[derive(Debug)]
pub(crate) struct AppRootLease {
    _file: File,
}

impl AppRootLease {
    pub(crate) fn acquire(root: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(root)?;
        let root = root.canonicalize()?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(".auralis-owner.lock"))?;
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "This Auralis storage is already open in another instance. Close it before retrying.",
            ),
            std::fs::TryLockError::Error(error) => error,
        })?;
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn independent_roots_and_stale_lock_files_are_safe() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let lease = AppRootLease::acquire(first.path()).unwrap();
        assert!(AppRootLease::acquire(first.path()).is_err());
        let _other = AppRootLease::acquire(second.path()).unwrap();
        drop(lease);
        assert!(first.path().join(".auralis-owner.lock").is_file());
        let _reacquired = AppRootLease::acquire(first.path()).unwrap();
    }

    #[test]
    fn another_process_cannot_own_the_same_root() {
        const KEY: &str = "AURALIS_LEASE_TEST_ROOT";
        if let Some(root) = std::env::var_os(KEY) {
            assert_eq!(
                AppRootLease::acquire(Path::new(&root)).unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let _lease = AppRootLease::acquire(root.path()).unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "bootstrap::app_root_lease::tests::another_process_cannot_own_the_same_root",
            ])
            .env(KEY, root.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
    }
}
