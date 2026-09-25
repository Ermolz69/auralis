use super::TranslationRunRequest;
use crate::error::ApplicationError;
use domain::job::{Job, JobKind};
use ports::error::PortError;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_links::TranslationLinkStore;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::sync::Arc;

const JOB_TITLE: &str = "Translate subtitles";

#[derive(Clone)]
pub struct StartTranslationJobUseCase {
    links: Arc<dyn TranslationLinkStore>,
    control: Arc<dyn TranslationRunControlPort>,
    jobs: Arc<dyn TranslationHostJobStore>,
}

impl StartTranslationJobUseCase {
    pub fn new(
        links: Arc<dyn TranslationLinkStore>,
        control: Arc<dyn TranslationRunControlPort>,
        jobs: Arc<dyn TranslationHostJobStore>,
    ) -> Self {
        Self {
            links,
            control,
            jobs,
        }
    }

    pub async fn execute(&self, request: TranslationRunRequest) -> Result<Job, ApplicationError> {
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
        let intent = self.links.run_intent(run_id).await?.ok_or_else(|| {
            ApplicationError::InvalidOperation {
                message: "Active translation has no frozen run intent".into(),
            }
        })?;
        if intent.translation_id != link.translation_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Frozen run belongs to another translation".into(),
            });
        }
        let snapshot = self.control.snapshot(&link).await?;
        if snapshot.run_id != run_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Translate run differs from the active project run".into(),
            });
        }
        if snapshot.phase == TranslationRunPhase::Validated {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation run already has a validated result".into(),
            });
        }
        if let Some(active) = self.jobs.active_for_run(run_id).await? {
            return Ok(active);
        }
        if snapshot.phase == TranslationRunPhase::Running {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation run already has an active attempt".into(),
            });
        }
        let job = Job::new(request.project_id, JOB_TITLE.into(), JobKind::Translation);
        match self
            .jobs
            .create_pending(job, request.translation_id, run_id)
            .await
        {
            Ok(job) => Ok(job),
            Err(error @ PortError::Conflict { .. }) => {
                match self.jobs.active_for_run(run_id).await? {
                    Some(active) => Ok(active),
                    None => Err(error.into()),
                }
            }
            Err(error) => Err(error.into()),
        }
    }
}
