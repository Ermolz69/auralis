use tokio::time::{Duration, sleep};

use crate::id::JobId;
use crate::manager::JobManager;
use crate::status::JobStatus;

#[tokio::test]
async fn test_job_manager_flow() {
    let manager = JobManager::new(None);

    // Start mock job
    let job_id = manager
        .start_mock_dubbing_job_internal("Test Job".to_string(), None)
        .await;

    // Should be queued or running
    let job = manager.get_job_internal(&job_id).await.unwrap();
    assert!(job.status == JobStatus::Queued || job.status == JobStatus::Running);

    // Cancel the job
    let _cancelled_job = manager.cancel_job_internal(&job_id).await.unwrap();

    // Wait a bit to ensure it processes the cancellation
    sleep(Duration::from_millis(600)).await;

    let job = manager.get_job_internal(&job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Cancelled);
}

#[tokio::test]
async fn test_cancel_unknown_job() {
    let manager = JobManager::new(None);
    let fake_id = JobId("fake".to_string());

    let result = manager.cancel_job_internal(&fake_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_updates_and_cancellation() {
    let manager = JobManager::new(None);
    let job_id = manager
        .start_mock_dubbing_job_internal("Concurrent Test".to_string(), None)
        .await;

    let manager_clone = manager.clone();
    let id_clone = job_id.clone();

    // Spawn tasks to spam updates
    let mut tasks = vec![];
    for _ in 0..10 {
        let m = manager.clone();
        let id = job_id.clone();
        tasks.push(tokio::spawn(async move {
            for j in 0..20 {
                m.update_job_progress(&id, j as u8).await;
                tokio::task::yield_now().await;
            }
        }));
    }

    // Task to cancel
    tasks.push(tokio::spawn(async move {
        sleep(Duration::from_millis(5)).await;
        let _ = manager_clone.cancel_job_internal(&id_clone).await;
    }));

    for t in tasks {
        let _ = t.await;
    }

    // The test succeeds if it doesn't deadlock.
    let job = manager.get_job_internal(&job_id).await.unwrap();
    assert!(
        job.status == JobStatus::Cancelled
            || job.status == JobStatus::Running
            || job.status == JobStatus::Queued
            || job.status == JobStatus::Completed
    );
}
