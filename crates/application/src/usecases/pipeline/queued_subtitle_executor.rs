use ports::{
    PortError,
    job_execution::{ExecutionContext, ExecutionDescriptor, JobExecutorPort, QueuedExecution},
    job_runtime_control::RuntimeTaskOutcome,
    repository::ProjectRepository,
    source::SubtitleSourcePort,
    storage::ArtifactStore,
};

use super::{
    mock_dubbing_pipeline::MockDubbingPipelineRunner, start_mock::StartMockPipelineDependencies,
};

pub(super) struct QueuedSubtitleExecutor<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    dependencies: StartMockPipelineDependencies<R, V, S>,
    inspector: Option<std::sync::Arc<dyn ports::translation_inspection::TranslationInspectorPort>>,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> QueuedSubtitleExecutor<R, V, S>
{
    pub fn new(
        dependencies: &StartMockPipelineDependencies<R, V, S>,
        inspector: Option<
            std::sync::Arc<dyn ports::translation_inspection::TranslationInspectorPort>,
        >,
    ) -> Self {
        Self {
            inspector,
            dependencies: StartMockPipelineDependencies {
                project_repo: dependencies.project_repo.clone(),
                job_scheduler: dependencies.job_scheduler.clone(),
                storage_uow: dependencies.storage_uow.clone(),
                subtitle_source: dependencies.subtitle_source.clone(),
                artifact_store: dependencies.artifact_store.clone(),
                workspace_port: dependencies.workspace_port.clone(),
                locks: dependencies.locks.clone(),
                job_runtime: dependencies.job_runtime.clone(),
            },
        }
    }
}

#[async_trait::async_trait]
impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> JobExecutorPort for QueuedSubtitleExecutor<R, V, S>
{
    fn validate(&self, descriptor: &ExecutionDescriptor) -> Result<(), PortError> {
        descriptor.validate()?;
        if descriptor.resources
            != ExecutionDescriptor::subtitle_import(descriptor.project_id.clone(), None).resources
        {
            return Err(PortError::InvalidResourceRequest {
                message: "Subtitle import v1 requires one CPU, network, disk and process unit"
                    .into(),
            });
        }
        Ok(())
    }

    async fn execute(
        &self,
        execution: QueuedExecution,
        context: ExecutionContext,
    ) -> RuntimeTaskOutcome {
        let dependencies = &self.dependencies;
        let descriptor = execution.descriptor;
        let runner = MockDubbingPipelineRunner::new(
            dependencies.job_scheduler.clone(),
            dependencies.project_repo.clone(),
            dependencies.subtitle_source.clone(),
            dependencies.storage_uow.clone(),
            dependencies.artifact_store.clone(),
            dependencies.workspace_port.clone(),
            dependencies.job_runtime.clone(),
        )
        .with_execution_inputs(descriptor, context.cpu, context.max_subtitle_bytes)
        .with_subtitle_inspector(self.inspector.clone());
        let span = tracing::info_span!("queued_subtitle_import", job_id = %execution.job.id(), project_id = %execution.job.project_id());
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span,
            crate::observability::execution_summary::OperationSummary::JobExecution {
                project_id: execution.job.project_id().to_string(),
                job_id: execution.job.id().to_string(),
                action: "job_execution",
                status: "aborted".into(),
            },
        );
        runner
            .run(
                execution.job.id().clone(),
                execution.job.project_id().clone(),
                context.cancellation,
                &mut guard,
            )
            .await
    }
}
