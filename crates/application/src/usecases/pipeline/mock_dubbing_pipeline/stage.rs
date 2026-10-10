use super::*;

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    T: StorageUnitOfWork + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> MockDubbingPipelineRunner<R, V, T, S>
{
    pub(super) async fn update_stage_or_terminalize(
        &self,
        job_id: &JobId,
        token: &CancellationToken,
        guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
        stage: DubbingPipelineStage,
        progress: JobProgress,
        failure: TerminalFailure,
    ) -> Result<(), RuntimeTaskOutcome> {
        match await_or_cancel(
            token,
            self.job_scheduler.update_job_stage(job_id, stage, progress),
        )
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(PortError::NotFound { .. })) => {
                guard.summary.update_status("deleted");
                Err(RuntimeTaskOutcome::DeletedNoOp)
            }
            Ok(Err(_)) => {
                Err(
                    terminalize_runner_failure(self.job_scheduler.as_ref(), job_id, failure, guard)
                        .await,
                )
            }
            Err(outcome) => {
                guard.summary.update_status("cancelled");
                Err(outcome)
            }
        }
    }
}
