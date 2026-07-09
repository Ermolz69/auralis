use std::time::Duration;
use tokio::time::sleep;

use crate::cancellation::CancelHandle;
use crate::manager::JobManager;
use domain::dubbing::DubbingPipelineStage;
use domain::job::{JobId, JobStatus};

pub fn run_mock_pipeline(manager: JobManager, job_id: JobId) {
    tokio::spawn(async move {
        let (handle, token) = CancelHandle::new();
        manager.register_cancel_handle(job_id.clone(), handle).await;

        let mut job = match manager.get_job_internal(&job_id).await {
            Some(j) => j,
            None => return, // Should not happen
        };

        // If it was cancelled before we even started
        if *job.status() == JobStatus::Cancelled {
            manager.remove_cancel_handle(&job_id).await;
            return;
        }

        let stages = vec![
            (DubbingPipelineStage::ValidateSource, 10, 500),
            (DubbingPipelineStage::FetchMetadata, 25, 600),
            (DubbingPipelineStage::DownloadMedia, 45, 800),
            (DubbingPipelineStage::ExtractOrGenerateTranscript, 70, 1000),
            (DubbingPipelineStage::ExportResult, 100, 500),
        ];

        for (stage, percent, delay_ms) in stages {
            sleep(Duration::from_millis(delay_ms)).await;

            if token.is_cancelled() {
                manager.cancel_job_internal(&job_id).await.ok();
                manager.remove_cancel_handle(&job_id).await;
                return;
            }

            job.update_stage(stage.clone()).ok();

            let mut progress = job.progress().clone();
            progress.percent = percent;
            progress.message = format!("Mock stage: {:?}", stage);
            job.update_progress(progress).ok();

            manager.update_job(job.clone()).await;
        }

        if job.mark_completed().is_ok() {
            manager.update_job(job.clone()).await;
        }
        manager.remove_cancel_handle(&job_id).await;
    });
}
