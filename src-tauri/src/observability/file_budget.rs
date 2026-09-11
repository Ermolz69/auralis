use super::bounded_writer::{DROPPED_EVENTS, WRITE_FAILURES};
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};
use tracing_appender::rolling::Rotation;

const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

pub(super) struct FileBudget {
    writer: Option<File>,
    directory: PathBuf,
    rotation: Rotation,
    period: String,
    used: u64,
    total: u64,
    limit: u64,
    max_files: usize,
    damaged: bool,
}

impl FileBudget {
    pub(super) fn new(directory: &Path, rotation: Rotation, max_files: usize) -> io::Result<Self> {
        let mut budget = Self {
            writer: None,
            directory: directory.into(),
            rotation,
            period: String::new(),
            used: 0,
            total: 0,
            limit: MAX_FILE_BYTES,
            max_files: max_files.clamp(1, 30),
            damaged: false,
        };
        budget.refresh()?;
        Ok(budget)
    }

    fn refresh(&mut self) -> io::Result<()> {
        let now = domain::chrono::Utc::now();
        let suffix = if self.rotation == Rotation::NEVER {
            String::new()
        } else if self.rotation == Rotation::MINUTELY {
            now.format(".%Y-%m-%d-%H-%M").to_string()
        } else if self.rotation == Rotation::HOURLY {
            now.format(".%Y-%m-%d-%H").to_string()
        } else if self.rotation == Rotation::WEEKLY {
            now.format(".%G-W%V").to_string()
        } else {
            now.format(".%Y-%m-%d").to_string()
        };
        self.open_period(format!("auralis.log{suffix}"))
    }

    fn open_period(&mut self, period: String) -> io::Result<()> {
        if self.period == period {
            return Ok(());
        }
        let path = self.directory.join(&period);
        let mut owned = Vec::new();
        for entry in std::fs::read_dir(&self.directory)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !is_owned_log(&name) {
                continue;
            }
            if !entry.file_type()?.is_file() {
                if name == period {
                    return Err(io::Error::other("Log destination is not a regular file"));
                }
                continue;
            }
            owned.push((name, entry.path(), entry.metadata()?.len()));
        }
        owned.sort_by(|a, b| a.0.cmp(&b.0));
        let creating = !owned.iter().any(|(name, _, _)| name == &period);
        let remove_count = (owned.len() + usize::from(creating)).saturating_sub(self.max_files);
        let mut removed = 0;
        let mut total = 0;
        for (name, old_path, length) in owned {
            if removed < remove_count && name != period {
                std::fs::remove_file(old_path)?;
                removed += 1;
            } else {
                total += length;
            }
        }
        let writer = OpenOptions::new().create(true).append(true).open(path)?;
        self.used = writer.metadata()?.len();
        self.total = total;
        self.writer = Some(writer);
        self.period = period;
        self.damaged = false;
        Ok(())
    }
}

fn is_owned_log(name: &str) -> bool {
    if name == "auralis.log" {
        return true;
    }
    let Some(suffix) = name.strip_prefix("auralis.log.") else {
        return false;
    };
    matches!(suffix.len(), 8 | 10 | 13 | 16)
        && suffix
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'-' || b == b'W')
}

impl Write for FileBudget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Err(error) = self.refresh() {
            WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
        if self.damaged
            || bytes.len() as u64 > self.limit.saturating_sub(self.used)
            || bytes.len() as u64 > (self.limit * self.max_files as u64).saturating_sub(self.total)
        {
            DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
            return Ok(bytes.len());
        }
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| io::Error::other("Log writer unavailable"))?;
        let result = writer.write_all(bytes);
        self.used = self.used.saturating_add(bytes.len() as u64);
        self.total = self.total.saturating_add(bytes.len() as u64);
        if let Err(error) = result {
            // Do not append subsequent JSON to an incomplete record.
            self.damaged = true;
            WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        match self.writer.as_mut() {
            Some(writer) => writer.flush().inspect_err(|_| {
                WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
            }),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn byte_quota_survives_reopen_and_drops_whole_records() {
        let directory = tempfile::tempdir().unwrap();
        for _ in 0..2 {
            let mut budget = FileBudget::new(directory.path(), Rotation::NEVER, 30).unwrap();
            budget.limit = 12;
            budget.write_all(b"{\"ok\":true}\n").unwrap();
            budget.write_all(b"{\"no\":true}\n").unwrap();
            budget.flush().unwrap();
        }
        assert_eq!(
            std::fs::read(directory.path().join("auralis.log")).unwrap(),
            b"{\"ok\":true}\n"
        );
    }
    #[test]
    fn rotation_failure_never_falls_back_to_a_full_old_file() {
        let directory = tempfile::tempdir().unwrap();
        let mut budget = FileBudget::new(directory.path(), Rotation::NEVER, 2).unwrap();
        budget.write_all(b"old\n").unwrap();
        let blocked = directory.path().join("auralis.log.2000-01-01");
        std::fs::create_dir(&blocked).unwrap();
        assert!(budget.open_period("auralis.log.2000-01-01".into()).is_err());
        assert_eq!(budget.period, "auralis.log");
        assert_eq!(
            std::fs::read(directory.path().join("auralis.log")).unwrap(),
            b"old\n"
        );
    }
    #[test]
    fn retention_only_prunes_owned_logs_and_respects_existing_total_bytes() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("keep.txt"), b"private").unwrap();
        std::fs::write(directory.path().join("auralis.log.2000-01-01"), b"old").unwrap();
        let mut budget = FileBudget::new(directory.path(), Rotation::NEVER, 1).unwrap();
        assert!(!directory.path().join("auralis.log.2000-01-01").exists());
        assert!(directory.path().join("keep.txt").exists());
        budget.limit = 4;
        budget.write_all(b"1234").unwrap();
        budget.write_all(b"5").unwrap();
        assert_eq!(
            std::fs::metadata(directory.path().join("auralis.log"))
                .unwrap()
                .len(),
            4
        );
    }
}
