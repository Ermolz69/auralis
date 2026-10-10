use super::support::*;
use domain::job::JobStatus;
use jobs::queue::JobQueueConfig;
use ports::{
    cancellation::CancellationToken,
    job_execution::{ExecutionContext, ExecutionDescriptor, JobExecutorPort, QueuedExecution},
    job_runtime_control::RuntimeTaskOutcome,
    job_scheduler::JobSchedulerPort,
};
use std::{sync::Arc, time::Duration};

struct CpuStarted {
    entered: CancellationToken,
    release: std::sync::mpsc::Sender<()>,
}

struct BlockingCpu {
    manager: Arc<jobs::manager::JobManager>,
    started: tokio::sync::mpsc::UnboundedSender<CpuStarted>,
}

#[async_trait::async_trait]
impl JobExecutorPort for BlockingCpu {
    fn validate(&self, descriptor: &ExecutionDescriptor) -> Result<(), ports::PortError> {
        descriptor.validate()
    }
    async fn execute(
        &self,
        execution: QueuedExecution,
        context: ExecutionContext,
    ) -> RuntimeTaskOutcome {
        let entered = CancellationToken::new();
        let ready = entered.clone();
        let (release, receiver) = std::sync::mpsc::channel();
        self.started.send(CpuStarted { entered, release }).unwrap();
        tokio::select! {
            result = context.cpu.run(Box::new(move || { ready.cancel(); receiver.recv().unwrap(); })) => {
                result.unwrap();
                self.manager.complete_job(execution.job.id()).await.unwrap();
                RuntimeTaskOutcome::Completed
            }
            _ = context.cancellation.cancelled() => RuntimeTaskOutcome::Cancelled,
        }
    }
}

#[tokio::test]
async fn dropped_cpu_future_cannot_release_capacity_before_the_closure_exits() {
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
    let (send, mut started) = tokio::sync::mpsc::unbounded_channel();
    harness
        .manager
        .start_dispatcher(Arc::new(BlockingCpu {
            manager: harness.manager.clone(),
            started: send,
        }))
        .await
        .unwrap();
    let first = harness.admit(CPU).await;
    let worker = tokio::time::timeout(Duration::from_secs(5), started.recv())
        .await
        .unwrap()
        .unwrap();
    worker.entered.cancelled().await;
    let second = harness.admit(CPU).await;
    let manager = harness.manager.clone();
    let first_id = first.clone();
    let cancel = tokio::spawn(async move { manager.cancel_job(&first_id).await });
    harness.wait_status(&first, JobStatus::Cancelling).await;
    assert!(!cancel.is_finished());
    assert_eq!(harness.status(&second).await, JobStatus::Pending);
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 0);
    worker.release.send(()).unwrap();
    assert_eq!(cancel.await.unwrap().unwrap().status, JobStatus::Cancelled);
    let worker = tokio::time::timeout(Duration::from_secs(5), started.recv())
        .await
        .unwrap()
        .unwrap();
    worker.entered.cancelled().await;
    worker.release.send(()).unwrap();
    harness.wait_status(&second, JobStatus::Completed).await;
    harness.shutdown().await;
}
