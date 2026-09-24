use super::{ExecuteTranslationRequest, ExecuteTranslationUseCase};
use crate::error::ApplicationError;
use domain::job::{JobError, JobId, JobStatus};
use domain::project::ProjectId;
use ports::committed_translation_progress::CommittedTranslationProgress;
use ports::error::PortError;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_links::TranslationLinkStore;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

const FAILURE_CODE: &str = "TRANSLATION_EXECUTION_FAILED";
const FAILURE_MESSAGE: &str = "Translation attempt failed; inspect run diagnostics";

pub struct RunTranslationHostJobUseCase {
    links: Arc<dyn TranslationLinkStore>,
    jobs: Arc<dyn TranslationHostJobStore>,
    executor: Arc<ExecuteTranslationUseCase>,
}

impl RunTranslationHostJobUseCase {
    pub fn new(
        links: Arc<dyn TranslationLinkStore>,
        jobs: Arc<dyn TranslationHostJobStore>,
        executor: Arc<ExecuteTranslationUseCase>,
    ) -> Self {
        Self {
            links,
            jobs,
            executor,
        }
    }

    pub async fn execute(
        &self,
        request: ExecuteTranslationRequest,
        job_id: &JobId,
    ) -> Result<Uuid, ApplicationError> {
        let link = self
            .links
            .get(request.translation_id)
            .await?
            .ok_or_else(|| PortError::NotFound {
                resource: "translation link".into(),
            })?;
        if link.project_id != request.project_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation belongs to another project".into(),
            });
        }
        let run_id = link
            .active_run_id
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Translation has no active run".into(),
            })?;
        let project_id = request.project_id.clone();
        let mut job = self.active_job(run_id, job_id, &project_id).await?;
        if job.status() != &JobStatus::Pending {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation host job is not pending".into(),
            });
        }
        let expected_revision = job.revision();
        job.start()?;
        self.jobs.start(&job, expected_revision).await?;

        let result = self
            .drive_execution(request, run_id, job_id, &project_id)
            .await;
        let mut current = self.active_job(run_id, job_id, &project_id).await?;
        let expected_revision = current.revision();
        match result {
            Ok(result_id) => {
                match current.status() {
                    JobStatus::Running => current.mark_completed()?,
                    JobStatus::Cancelling => current.cancel()?,
                    _ => {
                        return Err(ApplicationError::InvalidOperation {
                            message: "Translation host job changed state during execution".into(),
                        });
                    }
                }
                self.jobs.finish(&current, expected_revision).await?;
                Ok(result_id)
            }
            Err(error) => {
                if matches!(&error, ApplicationError::Port(PortError::Cancelled))
                    || current.status() == &JobStatus::Cancelling
                {
                    current.cancel()?;
                } else {
                    current.mark_failed(JobError::new(FAILURE_CODE, FAILURE_MESSAGE, true))?;
                }
                self.jobs.finish(&current, expected_revision).await?;
                Err(error)
            }
        }
    }

    async fn drive_execution(
        &self,
        request: ExecuteTranslationRequest,
        run_id: Uuid,
        job_id: &JobId,
        project_id: &ProjectId,
    ) -> Result<Uuid, ApplicationError> {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let execution = self.executor.execute_for_host(request, job_id, sender);
        tokio::pin!(execution);
        let mut receiver_open = true;
        let result = loop {
            tokio::select! {
                result = &mut execution => break result,
                progress = receiver.recv(), if receiver_open => {
                    match progress {
                        Some(progress) => self.log_progress(run_id, job_id, project_id, progress).await,
                        None => receiver_open = false,
                    }
                }
            }
        };
        while let Ok(progress) = receiver.try_recv() {
            self.log_progress(run_id, job_id, project_id, progress)
                .await;
        }
        result
    }

    async fn log_progress(
        &self,
        run_id: Uuid,
        job_id: &JobId,
        project_id: &ProjectId,
        progress: CommittedTranslationProgress,
    ) {
        if let Err(error) = self
            .persist_progress(run_id, job_id, project_id, progress)
            .await
        {
            tracing::warn!(
                job_id = %job_id,
                run_id = %run_id,
                error = %error,
                "committed translation progress could not be mirrored to the host job"
            );
        }
    }

    async fn persist_progress(
        &self,
        run_id: Uuid,
        job_id: &JobId,
        project_id: &ProjectId,
        progress: CommittedTranslationProgress,
    ) -> Result<(), ApplicationError> {
        if progress.run_id != run_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Committed progress belongs to another translation run".into(),
            });
        }
        let saved = u64::try_from(progress.saved_blocks).map_err(|_| {
            ApplicationError::InvalidOperation {
                message: "Committed translation block count exceeds host range".into(),
            }
        })?;
        let total = u64::try_from(progress.total_blocks).map_err(|_| {
            ApplicationError::InvalidOperation {
                message: "Translation block plan exceeds host range".into(),
            }
        })?;
        let mut job = self.active_job(run_id, job_id, project_id).await?;
        let expected_revision = job.revision();
        job.report_translation_progress(saved, total)?;
        self.jobs.record_progress(&job, expected_revision).await?;
        Ok(())
    }

    async fn active_job(
        &self,
        run_id: Uuid,
        job_id: &JobId,
        project_id: &ProjectId,
    ) -> Result<domain::job::Job, ApplicationError> {
        let job = self
            .jobs
            .active_for_run(run_id)
            .await?
            .ok_or_else(|| PortError::NotFound {
                resource: "active translation host job".into(),
            })?;
        if job.id() != job_id || job.project_id() != project_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Host job does not own the active translation run".into(),
            });
        }
        Ok(job)
    }
}
