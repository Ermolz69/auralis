use super::support::*;
use domain::job::JobStatus;
use jobs::queue::JobQueueConfig;
use ports::{
    job_execution::{ExecutionDescriptor, JobQueuePort},
    job_resources::ResourceDemand,
    transaction::CommitPipelineStart,
};

#[tokio::test]
async fn independent_jobs_execute_together_and_excess_work_stays_pending() {
    let config = JobQueueConfig {
        budgets: ResourceDemand {
            cpu: 2,
            network: 2,
            disk: 2,
            processes: 2,
        },
        ..Default::default()
    };
    let harness = Harness::new(config).await;
    let mut events = harness.controlled().await;
    let first = harness.admit(CPU).await;
    let first_run = started(&mut events).await;
    let second = harness.admit(CPU).await;
    let second_run = started(&mut events).await;
    assert_eq!(harness.status(&first).await, JobStatus::Running);
    assert_eq!(harness.status(&second).await, JobStatus::Running);
    let third = harness.admit(CPU).await;
    assert_eq!(harness.status(&third).await, JobStatus::Pending);
    first_run.finish.send(false).unwrap();
    let third_run = started(&mut events).await;
    assert_eq!(third_run.id, third);
    harness.wait_status(&first, JobStatus::Failed).await;
    second_run.finish.send(true).unwrap();
    third_run.finish.send(true).unwrap();
    harness.wait_status(&third, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn disjoint_work_bypasses_cpu_head_and_large_job_is_fenced_after_bounded_bypasses() {
    let config = JobQueueConfig {
        budgets: ResourceDemand {
            cpu: 2,
            network: 1,
            disk: 1,
            processes: 1,
        },
        max_bypasses: 2,
        ..Default::default()
    };
    let harness = Harness::new(config).await;
    let mut events = harness.controlled().await;
    let first = harness.admit(CPU).await;
    let first_run = started(&mut events).await;
    let large = harness
        .admit(ResourceDemand {
            cpu: 2,
            ..Default::default()
        })
        .await;
    let network = harness.admit(NETWORK).await;
    let network_run = started(&mut events).await;
    assert_eq!(network_run.id, network);
    assert_eq!(harness.status(&large).await, JobStatus::Pending);
    network_run.finish.send(true).unwrap();
    let small = harness.admit(CPU).await;
    let small_run = started(&mut events).await;
    assert_eq!(small_run.id, small);
    let younger = harness.admit(CPU).await;
    small_run.finish.send(true).unwrap();
    harness.wait_status(&small, JobStatus::Completed).await;
    // Releasing one CPU cannot admit more shared-category work after the fence.
    assert_eq!(harness.status(&younger).await, JobStatus::Pending);
    first_run.finish.send(true).unwrap();
    harness.wait_status(&first, JobStatus::Completed).await;
    let large_run = started(&mut events).await;
    assert_eq!(large_run.id, large);
    assert_eq!(harness.status(&younger).await, JobStatus::Pending);
    large_run.finish.send(true).unwrap();
    let younger_run = started(&mut events).await;
    assert_eq!(younger_run.id, younger);
    younger_run.finish.send(true).unwrap();
    harness.wait_status(&younger, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn impossible_requests_and_overflow_leave_no_partial_admission() {
    let config = JobQueueConfig {
        budgets: ResourceDemand {
            cpu: 1,
            network: 1,
            disk: 1,
            processes: 1,
        },
        pending_capacity: 1,
        dispatch_window: 1,
        ..Default::default()
    };
    let harness = Harness::new(config).await;
    let mut events = harness.controlled().await;
    harness.admit(CPU).await;
    let running = started(&mut events).await;
    harness.admit(CPU).await;
    let mut project = harness.project().await;
    let original = project.clone();
    let job = domain::job::Job::new(
        project.id().clone(),
        "Rejected".into(),
        domain::job::JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    let mut descriptor = ExecutionDescriptor::subtitle_import(project.id().clone(), None);
    descriptor.resources = ResourceDemand {
        cpu: 2,
        ..Default::default()
    };
    let result = harness
        .manager
        .admit_pipeline(
            CommitPipelineStart {
                project: project.clone(),
                job: job.clone(),
            },
            descriptor.clone(),
        )
        .await;
    assert!(matches!(
        result,
        Err(ports::PortError::InvalidResourceRequest { .. })
    ));
    descriptor.resources = CPU;
    let result = harness
        .manager
        .admit_pipeline(
            CommitPipelineStart {
                project,
                job: job.clone(),
            },
            descriptor,
        )
        .await;
    assert!(matches!(
        result,
        Err(ports::PortError::QueueFull { capacity: 1 })
    ));
    use ports::repository::{JobRepository, ProjectRepository};
    assert!(harness.jobs.get(job.id()).await.unwrap().is_none());
    assert_eq!(
        harness.projects.get(original.id()).await.unwrap().unwrap(),
        original
    );
    running.finish.send(true).unwrap();
    let waiting = started(&mut events).await;
    waiting.finish.send(true).unwrap();
    harness.wait_status(&waiting.id, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn separate_legacy_translation_job_does_not_block_pipeline_admission() {
    use ports::{
        job_execution::JobQueuePort, repository::JobRepository, transaction::CommitPipelineStart,
    };
    let harness = Harness::new(JobQueueConfig::default()).await;
    let mut events = harness.controlled().await;
    let mut project = harness.project().await;
    let translation = domain::job::Job::new(
        project.id().clone(),
        "Legacy translation".into(),
        domain::job::JobKind::Translation,
    );
    harness.jobs.create(translation.clone()).await.unwrap();
    let pipeline = domain::job::Job::new(
        project.id().clone(),
        "Pipeline".into(),
        domain::job::JobKind::Dubbing,
    );
    project.start_processing(pipeline.id().clone()).unwrap();
    let mut descriptor =
        ports::job_execution::ExecutionDescriptor::subtitle_import(project.id().clone(), None);
    descriptor.resources = CPU;
    harness
        .manager
        .admit_pipeline(
            CommitPipelineStart {
                project,
                job: pipeline.clone(),
            },
            descriptor,
        )
        .await
        .unwrap();
    let started = started(&mut events).await;
    assert_eq!(started.id, *pipeline.id());
    started.finish.send(true).unwrap();
    harness
        .wait_status(pipeline.id(), JobStatus::Completed)
        .await;
    assert_eq!(harness.status(translation.id()).await, JobStatus::Pending);
    harness.shutdown().await;
}
