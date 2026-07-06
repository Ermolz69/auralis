use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::cancellation::CancelHandle;
use crate::id::JobId;
use crate::manager::JobManager;
use crate::status::JobStatus;
use crate::job::Job;

#[tokio::test]
async fn test_job_manager_flow() {
    let manager = JobManager::new(None);
    
    // Start mock job
    let job_id = manager.start_mock_dubbing_job("Test Job".to_string(), None).await;
    
    // Should be queued or running
    let job = manager.get_job(&job_id).await.unwrap();
    assert!(job.status == JobStatus::Queued || job.status == JobStatus::Running);
    
    // Cancel the job
    let cancelled_job = manager.cancel_job(&job_id).await.unwrap();
    
    // Wait a bit to ensure it processes the cancellation
    sleep(Duration::from_millis(600)).await;
    
    let job = manager.get_job(&job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Cancelled);
}

#[tokio::test]
async fn test_cancel_unknown_job() {
    let manager = JobManager::new(None);
    let fake_id = JobId("fake".to_string());
    
    let result = manager.cancel_job(&fake_id).await;
    assert!(result.is_err());
}
