use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use domain::job::{Job, JobId};
use domain::project::ProjectId;
use jobs::manager::JobManager;
use ports::error::PortError;
use ports::job_scheduler::{JobSchedulerPort, StartDubbingJobRequest};
use ports::repository::JobRepository;
use ports::transaction::{
    ApplyTerminalLifecycle, CommitArtifactFinalize, CommitArtifactFinalizeResult, CommitJobUpdate,
    CommitManagedSourceImport, CommitPipelineStart, CommitPipelineStartFailure,
    CommitProjectDelete, CommitProjectDeleteResult, CommitStagedArtifactWrite,
    CommitTerminalJobUpdate, CommitTranscriptImport, CommitYoutubeImport, StorageUnitOfWork,
};
use serde::Serialize;

struct TrackingAllocator;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            if new_size >= layout.size() {
                let live = LIVE_BYTES.fetch_add(new_size - layout.size(), Ordering::Relaxed)
                    + new_size
                    - layout.size();
                PEAK_BYTES.fetch_max(live, Ordering::Relaxed);
            } else {
                LIVE_BYTES.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        resized
    }
}

fn record_allocation(size: usize) {
    let live = LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
    PEAK_BYTES.fetch_max(live, Ordering::Relaxed);
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
}

#[derive(Default)]
struct EphemeralJobStore {
    jobs: Mutex<HashMap<JobId, Job>>,
}

impl EphemeralJobStore {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<JobId, Job>>, PortError> {
        self.jobs.lock().map_err(|_| PortError::Storage {
            operation: "job_heap_soak",
            message: "ephemeral job store lock was poisoned".to_string(),
        })
    }

    fn len(&self) -> Result<usize, PortError> {
        Ok(self.lock()?.len())
    }
}

#[async_trait]
impl JobRepository for EphemeralJobStore {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        self.lock()?.insert(job.id().clone(), job.clone());
        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        Ok(self.lock()?.get(id).cloned())
    }

    async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        let mut jobs = self.lock()?;
        let existing = jobs.get(job.id()).ok_or_else(|| PortError::NotFound {
            resource: format!("Job {}", job.id()),
        })?;
        if existing.revision() != expected_revision {
            return Err(PortError::Conflict {
                resource: format!("Job {}", job.id()),
                message: "revision changed during heap soak".to_string(),
            });
        }
        jobs.insert(job.id().clone(), job.clone());
        Ok(())
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
        Ok(self
            .lock()?
            .values()
            .filter(|job| job.project_id() == project_id)
            .cloned()
            .collect())
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        Ok(self.lock()?.values().cloned().collect())
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        Ok(self.lock()?.values().take(limit).cloned().collect())
    }
}

struct EphemeralStorageUnitOfWork {
    store: Arc<EphemeralJobStore>,
}

#[async_trait]
impl StorageUnitOfWork for EphemeralStorageUnitOfWork {
    async fn commit_youtube_import(&self, _command: CommitYoutubeImport) -> Result<(), PortError> {
        unsupported()
    }

    async fn commit_transcript_import(
        &self,
        _command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        unsupported()
    }

    async fn commit_staged_artifact_write(
        &self,
        _command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        unsupported()
    }

    async fn commit_managed_source_import(
        &self,
        _command: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        unsupported()
    }

    async fn commit_project_delete(
        &self,
        _command: CommitProjectDelete,
    ) -> Result<CommitProjectDeleteResult, PortError> {
        unsupported()
    }

    async fn commit_job_update(&self, _command: CommitJobUpdate) -> Result<(), PortError> {
        unsupported()
    }

    async fn commit_pipeline_start(&self, _command: CommitPipelineStart) -> Result<(), PortError> {
        unsupported()
    }

    async fn commit_pipeline_start_failure(
        &self,
        _command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        unsupported()
    }

    async fn commit_terminal_job_update(
        &self,
        command: CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        let mut jobs = self.store.lock()?;
        let existing = jobs
            .get(command.job.id())
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", command.job.id()),
            })?;
        if existing.revision() != command.expected_revision {
            return Err(PortError::Conflict {
                resource: format!("Job {}", command.job.id()),
                message: "revision changed during terminal heap soak commit".to_string(),
            });
        }
        jobs.remove(command.job.id());
        Ok(())
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        _command: ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        unsupported()
    }

    async fn commit_artifact_finalize(
        &self,
        _command: CommitArtifactFinalize,
    ) -> Result<CommitArtifactFinalizeResult, PortError> {
        unsupported()
    }
}

fn unsupported<T>() -> Result<T, PortError> {
    Err(PortError::Unsupported {
        message: "operation is outside the job heap soak workload".to_string(),
    })
}

#[derive(Default)]
struct Regression {
    count: f64,
    sum_x: f64,
    sum_y: f64,
    sum_xy: f64,
    sum_x_squared: f64,
}

impl Regression {
    fn sample(&mut self, iteration: usize, live_delta: i64) {
        let x = iteration as f64;
        let y = live_delta as f64;
        self.count += 1.0;
        self.sum_x += x;
        self.sum_y += y;
        self.sum_xy += x * y;
        self.sum_x_squared += x * x;
    }

    fn slope(&self) -> f64 {
        let denominator = self.count * self.sum_x_squared - self.sum_x * self.sum_x;
        if denominator.abs() < f64::EPSILON {
            0.0
        } else {
            (self.count * self.sum_xy - self.sum_x * self.sum_y) / denominator
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeapSoakReport {
    duration_seconds: f64,
    iterations: usize,
    baseline_live_bytes: usize,
    final_live_bytes: usize,
    peak_live_bytes: usize,
    retained_bytes: i64,
    slope_bytes_per_iteration: f64,
    allocations: usize,
    deallocations: usize,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "long-running heap profiler; run with task rs:soak:jobs"]
async fn job_lifecycle_heap_remains_bounded() -> Result<(), Box<dyn Error>> {
    let duration = Duration::from_secs(env_usize("AURALIS_JOB_SOAK_SECONDS", 600)? as u64);
    let minimum_iterations = env_usize("AURALIS_JOB_SOAK_MIN_ITERATIONS", 100_000)?;
    let sample_every = env_usize("AURALIS_JOB_SOAK_SAMPLE_EVERY", 1_000)?.max(1);
    let maximum_retained_bytes = env_usize("AURALIS_JOB_SOAK_MAX_RETAINED_BYTES", 1_048_576)?;
    let maximum_slope = env_f64("AURALIS_JOB_SOAK_MAX_SLOPE_BYTES", 8.0)?;

    let store = Arc::new(EphemeralJobStore::default());
    let unit_of_work = Arc::new(EphemeralStorageUnitOfWork {
        store: store.clone(),
    });
    let manager = JobManager::new(store.clone(), unit_of_work, None);
    let project_id = ProjectId::new();

    for iteration in 0..5_000 {
        run_lifecycle(&manager, &project_id, iteration).await?;
    }
    tokio::task::yield_now().await;

    let baseline_live_bytes = LIVE_BYTES.load(Ordering::Relaxed);
    PEAK_BYTES.store(baseline_live_bytes, Ordering::Relaxed);
    let baseline_allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let baseline_deallocations = DEALLOCATIONS.load(Ordering::Relaxed);
    let started_at = Instant::now();
    let mut iteration = 0;
    let mut regression = Regression::default();

    while started_at.elapsed() < duration || iteration < minimum_iterations {
        run_lifecycle(&manager, &project_id, iteration).await?;
        iteration += 1;
        if iteration % sample_every == 0 {
            tokio::task::yield_now().await;
            if store.len()? != 0 {
                return Err("terminal job remained in the ephemeral store".into());
            }
            regression.sample(
                iteration,
                signed_delta(LIVE_BYTES.load(Ordering::Relaxed), baseline_live_bytes),
            );
        }
    }

    tokio::task::yield_now().await;
    let final_live_bytes = LIVE_BYTES.load(Ordering::Relaxed);
    let report = HeapSoakReport {
        duration_seconds: started_at.elapsed().as_secs_f64(),
        iterations: iteration,
        baseline_live_bytes,
        final_live_bytes,
        peak_live_bytes: PEAK_BYTES.load(Ordering::Relaxed),
        retained_bytes: signed_delta(final_live_bytes, baseline_live_bytes),
        slope_bytes_per_iteration: regression.slope(),
        allocations: ALLOCATIONS.load(Ordering::Relaxed) - baseline_allocations,
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed) - baseline_deallocations,
    };

    let report_json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = std::env::var_os("AURALIS_JOB_SOAK_REPORT").map(PathBuf::from) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &report_json)?;
    }
    println!("{report_json}");

    if report.retained_bytes > usize_to_i64(maximum_retained_bytes) {
        return Err(format!(
            "job lifecycle retained {} bytes; limit is {maximum_retained_bytes}",
            report.retained_bytes
        )
        .into());
    }
    if report.slope_bytes_per_iteration > maximum_slope {
        return Err(format!(
            "job lifecycle heap slope is {:.3} bytes/iteration; limit is {maximum_slope:.3}",
            report.slope_bytes_per_iteration
        )
        .into());
    }
    if store.len()? != 0 {
        return Err("job store is not empty after the soak".into());
    }

    Ok(())
}

async fn run_lifecycle(
    manager: &JobManager,
    project_id: &ProjectId,
    iteration: usize,
) -> Result<(), PortError> {
    let job = manager
        .start_dubbing_job(StartDubbingJobRequest {
            project_id: Some(project_id.clone()),
            title: "Heap soak job".to_string(),
        })
        .await?;
    if iteration.is_multiple_of(2) {
        manager.cancel_job(&job.id).await?;
    } else {
        manager.complete_job(&job.id).await?;
    }
    Ok(())
}

fn env_usize(name: &str, default: usize) -> Result<usize, Box<dyn Error>> {
    match std::env::var(name) {
        Ok(value) => Ok(value.parse()?),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error.into()),
    }
}

fn env_f64(name: &str, default: f64) -> Result<f64, Box<dyn Error>> {
    match std::env::var(name) {
        Ok(value) => Ok(value.parse()?),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error.into()),
    }
}

fn signed_delta(current: usize, baseline: usize) -> i64 {
    if current >= baseline {
        usize_to_i64(current - baseline)
    } else {
        -usize_to_i64(baseline - current)
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
