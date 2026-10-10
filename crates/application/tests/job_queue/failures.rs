use super::support::*;
use domain::job::JobStatus;
use jobs::queue::JobQueueConfig;
use ports::{
    job_execution::{ExecutionContext, ExecutionDescriptor, JobExecutorPort, QueuedExecution},
    job_runtime_control::RuntimeTaskOutcome,
    job_scheduler::JobSchedulerPort,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[tokio::test]
async fn initial_cancel_write_failure_does_not_signal_worker_and_terminal_failure_is_retryable() {
    let harness = Harness::new(JobQueueConfig::default()).await;
    let mut events = harness.controlled().await;
    let id = harness.admit(CPU).await;
    let running = started(&mut events).await;
    sqlx::raw_sql("CREATE TRIGGER reject_cancel BEFORE UPDATE ON jobs WHEN NEW.status = 'cancelling' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").execute(&harness.pool).await.unwrap();
    assert!(harness.manager.cancel_job(&id).await.is_err());
    assert_eq!(harness.status(&id).await, JobStatus::Running);
    sqlx::raw_sql("DROP TRIGGER reject_cancel; CREATE TRIGGER reject_terminal BEFORE UPDATE ON jobs WHEN NEW.status = 'cancelled' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").execute(&harness.pool).await.unwrap();
    running.cleanup.send(()).unwrap();
    assert!(harness.manager.cancel_job(&id).await.is_err());
    assert_eq!(harness.status(&id).await, JobStatus::Cancelling);
    sqlx::raw_sql("DROP TRIGGER reject_terminal;")
        .execute(&harness.pool)
        .await
        .unwrap();
    let terminal = harness.manager.cancel_job(&id).await.unwrap();
    assert_eq!(terminal.status, JobStatus::Cancelled);
    assert_eq!(
        terminal.revision,
        harness.manager.cancel_job(&id).await.unwrap().revision
    );
    let outbox: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM outbox_messages WHERE deduplication_key LIKE 'terminal-%'",
    )
    .fetch_one(&harness.pool)
    .await
    .unwrap();
    assert_eq!(outbox, 1);
    harness.shutdown().await;
}

struct PanicOnce {
    panic: AtomicBool,
    inner: ControlledExecutor,
}

#[async_trait::async_trait]
impl JobExecutorPort for PanicOnce {
    fn validate(&self, descriptor: &ExecutionDescriptor) -> Result<(), ports::PortError> {
        descriptor.validate()
    }
    async fn execute(
        &self,
        execution: QueuedExecution,
        context: ExecutionContext,
    ) -> RuntimeTaskOutcome {
        if self.panic.swap(false, Ordering::AcqRel) {
            let result = context
                .cpu
                .run(Box::new(|| panic!("CPU fixture panic")))
                .await;
            assert!(result.is_err());
            panic!("Executor fixture panic");
        }
        self.inner.execute(execution, context).await
    }
}

#[tokio::test]
async fn cpu_and_executor_panic_free_resources_and_preserve_the_pool() {
    let harness = Harness::new(JobQueueConfig {
        budgets: ports::job_resources::ResourceDemand {
            cpu: 1,
            network: 1,
            disk: 1,
            processes: 1,
        },
        ..Default::default()
    })
    .await;
    let (send, mut events) = tokio::sync::mpsc::unbounded_channel();
    harness
        .manager
        .start_dispatcher(Arc::new(PanicOnce {
            panic: AtomicBool::new(true),
            inner: ControlledExecutor {
                manager: harness.manager.clone(),
                started: send,
            },
        }))
        .await
        .unwrap();
    let first = harness.admit(CPU).await;
    harness.wait_status(&first, JobStatus::Failed).await;
    let second = harness.admit(CPU).await;
    let running = started(&mut events).await;
    assert_eq!(running.id, second);
    running.finish.send(true).unwrap();
    harness.wait_status(&second, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn failed_running_write_stops_gated_executor_and_releases_every_reservation() {
    use domain::job::{Job, JobKind};
    use ports::{job_execution::JobExecutionStore, transaction::CommitPipelineStart};
    let harness = Harness::new(JobQueueConfig {
        budgets: ports::job_resources::ResourceDemand {
            cpu: 1,
            network: 1,
            disk: 1,
            processes: 1,
        },
        ..Default::default()
    })
    .await;
    let mut project = harness.project().await;
    let job = Job::new(
        project.id().clone(),
        "Start failure".into(),
        JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    let mut descriptor = ExecutionDescriptor::subtitle_import(project.id().clone(), None);
    descriptor.resources = CPU;
    harness
        .store
        .admit_pipeline(
            CommitPipelineStart {
                project,
                job: job.clone(),
            },
            descriptor,
            16,
        )
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TRIGGER reject_start BEFORE UPDATE ON jobs WHEN NEW.status = 'running' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").execute(&harness.pool).await.unwrap();
    let mut events = harness.controlled().await;
    harness.wait_status(job.id(), JobStatus::Failed).await;
    sqlx::raw_sql("DROP TRIGGER reject_start;")
        .execute(&harness.pool)
        .await
        .unwrap();
    let next = harness.admit(CPU).await;
    let running = started(&mut events).await;
    assert_eq!(running.id, next);
    running.finish.send(true).unwrap();
    harness.wait_status(&next, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn executor_panic_with_process_ownership_quarantines_capacity() {
    let harness = Harness::new(JobQueueConfig::default()).await;
    let (send, _) = tokio::sync::mpsc::unbounded_channel();
    harness
        .manager
        .start_dispatcher(Arc::new(PanicOnce {
            panic: AtomicBool::new(true),
            inner: ControlledExecutor {
                manager: harness.manager.clone(),
                started: send,
            },
        }))
        .await
        .unwrap();
    let id = harness
        .admit(ports::job_resources::ResourceDemand {
            cpu: 1,
            processes: 1,
            ..Default::default()
        })
        .await;
    harness.wait_status(&id, JobStatus::Cancelling).await;
    assert_eq!(
        harness
            .manager
            .queue_available_resources()
            .unwrap()
            .processes,
        1
    );
    assert!(harness.manager.cancel_job(&id).await.is_err());
    let report = ports::job_runtime_control::JobRuntimeControlPort::drain_all(
        harness.manager.as_ref(),
        std::time::Duration::from_secs(1),
    )
    .await
    .unwrap();
    assert_eq!(report.unconfirmed_count, 1);
}

#[tokio::test]
async fn invalid_descriptor_in_dispatch_window_does_not_block_valid_work() {
    use domain::job::{Job, JobKind};
    use ports::{job_execution::JobExecutionStore, transaction::CommitPipelineStart};
    let harness = Harness::new(JobQueueConfig::default()).await;
    let mut project = harness.project().await;
    let job = Job::new(
        project.id().clone(),
        "Invalid descriptor".into(),
        JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    let mut descriptor = ExecutionDescriptor::subtitle_import(project.id().clone(), None);
    descriptor.resources = CPU;
    harness
        .store
        .admit_pipeline(
            CommitPipelineStart {
                project,
                job: job.clone(),
            },
            descriptor,
            16,
        )
        .await
        .unwrap();
    sqlx::query("UPDATE job_executions SET descriptor_json = '{}' WHERE job_id = ?")
        .bind(job.id().to_string())
        .execute(&harness.pool)
        .await
        .unwrap();
    let mut events = harness.controlled().await;
    harness.wait_status(job.id(), JobStatus::Failed).await;
    let next = harness.admit(CPU).await;
    let running = started(&mut events).await;
    assert_eq!(running.id, next);
    running.finish.send(true).unwrap();
    harness.wait_status(&next, JobStatus::Completed).await;
    harness.shutdown().await;
}
