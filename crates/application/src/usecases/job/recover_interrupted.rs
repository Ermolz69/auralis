use domain::job::{JobError, JobStatus};
use ports::repository::JobRepository;
use std::sync::Arc;

pub struct RecoverInterruptedJobsUseCase {
    job_repo: Arc<dyn JobRepository>,
}

impl RecoverInterruptedJobsUseCase {
    pub fn new(job_repo: Arc<dyn JobRepository>) -> Self {
        Self { job_repo }
    }

    pub async fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let active_jobs = self.job_repo.list_active().await?;

        for mut job in active_jobs {
            if matches!(*job.status(), JobStatus::Running | JobStatus::Pending) {
                let error =
                    JobError::new("APP_RESTART", "Interrupted by application restart", false);

                if let Err(e) = job.mark_failed(error) {
                    println!(
                        "Failed to mark interrupted job {} as failed: {}",
                        job.id(),
                        e
                    );
                    continue;
                }

                if let Err(e) = self.job_repo.save(&job).await {
                    println!("Failed to save interrupted job {}: {}", job.id(), e);
                } else {
                    println!(
                        "Recovered interrupted job {} by marking it as failed (restart)",
                        job.id()
                    );
                }
            }
        }

        Ok(())
    }
}
