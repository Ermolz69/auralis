use chrono::Utc;
use std::time::Duration;
use tokio::time::sleep;

use crate::cancellation::CancelHandle;
use crate::id::JobId;
use crate::manager::JobManager;
use crate::progress::JobProgress;
use crate::stage::JobStage;
use crate::status::JobStatus;

pub fn run_mock_pipeline(manager: JobManager, job_id: JobId) {
    tokio::spawn(async move {
        let (handle, token) = CancelHandle::new();
        manager.register_cancel_handle(job_id.clone(), handle).await;

        let mut job = match manager.get_job(&job_id).await {
            Some(j) => j,
            None => return, // Should not happen
        };

        // If it was cancelled before we even started
        if job.status == JobStatus::Cancelled {
            manager.remove_cancel_handle(&job_id).await;
            return;
        }

        job.status = JobStatus::Running;
        job.updated_at = Utc::now();
        manager.update_job(job.clone()).await;

        let stages = vec![
            (JobStage::ValidateSource, 10, 500),
            (JobStage::FetchMetadata, 25, 600),
            (JobStage::PrepareMedia, 45, 800),
            (JobStage::GenerateTranscript, 70, 1000),
            (JobStage::Finalize, 100, 500),
        ];

        for (stage, percent, delay_ms) in stages {
            sleep(Duration::from_millis(delay_ms)).await;

            if token.is_cancelled() {
                job.status = JobStatus::Cancelled;
                job.updated_at = Utc::now();
                manager.update_job(job.clone()).await;
                manager.remove_cancel_handle(&job_id).await;
                return;
            }

            job.stage = Some(stage);
            job.progress = JobProgress::new(percent);
            job.updated_at = Utc::now();
            manager.update_job(job.clone()).await;
        }

        job.status = JobStatus::Completed;
        job.updated_at = Utc::now();
        manager.update_job(job.clone()).await;
        manager.remove_cancel_handle(&job_id).await;
    });
}
