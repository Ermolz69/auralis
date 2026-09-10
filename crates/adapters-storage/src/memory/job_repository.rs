use async_trait::async_trait;

use std::sync::{Arc, Mutex};

use domain::job::{Job, JobId};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::JobRepository;

use super::database::InMemoryDatabase;

#[derive(Clone)]
pub struct InMemoryJobRepository {
    pub db: Arc<Mutex<InMemoryDatabase>>,
}

impl InMemoryJobRepository {
    pub fn new(db: Arc<Mutex<InMemoryDatabase>>) -> Self {
        Self { db }
    }

    fn lock_db(&self) -> Result<std::sync::MutexGuard<'_, InMemoryDatabase>, PortError> {
        self.db.lock().map_err(|_| PortError::Storage {
            operation: "lock_in_memory_db",
            message: "Mutex poisoned".to_string(),
        })
    }
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        let mut lock = self.lock_db()?;
        if lock.jobs.contains_key(job.id()) {
            return Err(PortError::Conflict {
                resource: "Job".to_string(),
                message: format!("Job with id {} already exists", job.id()),
            });
        }
        lock.jobs.insert(job.id().clone(), job.clone());
        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        let lock = self.lock_db()?;
        Ok(lock.jobs.get(id).cloned())
    }

    async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        let mut db = self.lock_db()?;

        let existing = db.jobs.get(job.id()).ok_or_else(|| PortError::Unexpected {
            message: format!("Job {} not found during save", job.id()),
        })?;

        if existing.revision() != expected_revision {
            return Err(PortError::Conflict {
                resource: "Job".to_string(),
                message: format!("Optimistic concurrency conflict for job id {}", job.id()),
            });
        }

        db.jobs.insert(job.id().clone(), job.clone());
        Ok(())
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
        let lock = self.lock_db()?;
        Ok(lock
            .jobs
            .values()
            .filter(|j| j.project_id() == project_id)
            .cloned()
            .collect())
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let lock = self.lock_db()?;
        Ok(lock
            .jobs
            .values()
            .filter(|job| {
                matches!(
                    job.status(),
                    domain::job::JobStatus::Pending
                        | domain::job::JobStatus::Running
                        | domain::job::JobStatus::Cancelling
                )
            })
            .cloned()
            .collect())
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        let lock = self.lock_db()?;
        let mut jobs: Vec<Job> = lock.jobs.values().cloned().collect();
        jobs.sort_by_key(|b| std::cmp::Reverse(*b.created_at()));
        Ok(jobs.into_iter().take(limit).collect())
    }
}

#[async_trait]
impl ports::job_query::JobQueryPort for InMemoryJobRepository {
    async fn list_jobs_snapshot(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ports::job_scheduler::ScheduledJob>, PortError> {
        let lock = self.lock_db()?;
        let jobs: Vec<_> = lock
            .jobs
            .values()
            .filter(|j| j.project_id() == project_id)
            .cloned()
            .collect();

        let mut dtos = Vec::with_capacity(jobs.len());
        for job in jobs {
            let snap = job.to_snapshot();
            dtos.push(ports::job_scheduler::ScheduledJob {
                id: snap.id,
                kind: snap.kind,
                revision: snap.revision,
                project_id: Some(snap.project_id),
                title: snap.title,
                status: snap.status,
                stage: snap.stage,
                progress: snap.progress,
                error: snap.error.map(|e| e.message),
                created_at: snap.created_at,
                updated_at: snap.updated_at,
            });
        }
        Ok(dtos)
    }

    async fn list_job_history_page(
        &self,
        cursor: Option<&ports::job_query::JobHistoryCursor>,
        limit: usize,
    ) -> Result<ports::job_query::JobHistoryPage, PortError> {
        let lock = self.lock_db()?;
        let mut jobs: Vec<_> = lock
            .jobs
            .values()
            .filter(|job| {
                matches!(
                    job.status(),
                    domain::job::JobStatus::Completed
                        | domain::job::JobStatus::Failed
                        | domain::job::JobStatus::Cancelled
                )
            })
            .filter(|job| {
                cursor.is_none_or(|cursor| {
                    job.created_at() < &cursor.created_at
                        || (job.created_at() == &cursor.created_at
                            && job.id().to_string() < cursor.job_id.to_string())
                })
            })
            .cloned()
            .collect();
        jobs.sort_by(|left, right| {
            right
                .created_at()
                .cmp(left.created_at())
                .then_with(|| right.id().to_string().cmp(&left.id().to_string()))
        });

        build_history_page(jobs, limit)
    }
}

fn build_history_page(
    jobs: Vec<Job>,
    limit: usize,
) -> Result<ports::job_query::JobHistoryPage, PortError> {
    let page_size = limit.clamp(1, ports::job_query::MAX_JOB_HISTORY_PAGE_SIZE);
    let has_more = jobs.len() > page_size;
    let dtos = jobs
        .into_iter()
        .take(page_size)
        .map(job_to_scheduled)
        .collect::<Vec<_>>();
    let next_cursor = if has_more {
        dtos.last().map(|last| ports::job_query::JobHistoryCursor {
            created_at: last.created_at,
            job_id: last.id.clone(),
        })
    } else {
        None
    };

    Ok(ports::job_query::JobHistoryPage {
        jobs: dtos,
        next_cursor,
    })
}

fn job_to_scheduled(job: Job) -> ports::job_scheduler::ScheduledJob {
    let snap = job.to_snapshot();
    ports::job_scheduler::ScheduledJob {
        id: snap.id,
        kind: snap.kind,
        revision: snap.revision,
        project_id: Some(snap.project_id),
        title: snap.title,
        status: snap.status,
        stage: snap.stage,
        progress: snap.progress,
        error: snap.error.map(|error| error.message),
        created_at: snap.created_at,
        updated_at: snap.updated_at,
    }
}
