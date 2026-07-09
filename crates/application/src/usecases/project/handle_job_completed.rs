use crate::error::ApplicationError;
use ports::repository::ProjectRepository;
use std::str::FromStr;

pub struct HandleJobCompletedRequest {
    pub job_id: String,
    pub project_id: String,
    pub is_success: bool,
}

pub struct HandleJobCompletedResult {
    pub transcript_ready: bool,
}

pub struct HandleJobCompletedUseCase<R: ProjectRepository + Clone + 'static> {
    project_repo: R,
}

impl<R: ProjectRepository + Clone + 'static> HandleJobCompletedUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        req: HandleJobCompletedRequest,
    ) -> Result<HandleJobCompletedResult, ApplicationError> {
        let pid = domain::project::ProjectId::from_str(&req.project_id).map_err(|e| {
            ApplicationError::InvalidOperation {
                message: e.to_string(),
            }
        })?;

        let mut project = self
            .project_repo
            .get(&pid)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(pid.clone()))?;

        let transcript_ready = project.transcript().is_some();

        if req.is_success {
            project.mark_completed()?;
        } else {
            project.mark_failed()?;
        }

        self.project_repo.save(&project).await?;

        Ok(HandleJobCompletedResult { transcript_ready })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::project::{Project, ProjectId};
    use ports::error::PortError;

    #[derive(Clone)]
    struct FailingSaveRepo;

    #[async_trait]
    impl ProjectRepository for FailingSaveRepo {
        async fn get(&self, _id: &ProjectId) -> Result<Option<Project>, PortError> {
            let mut project = Project::new("Test".into());
            // Need processing state to pass the transition check
            project
                .import_source(
                    domain::media::MediaSource::LocalFile { path: "".into() },
                    None,
                )
                .unwrap();
            project.mark_ready_for_processing().unwrap();
            project.mark_processing_started().unwrap();
            Ok(Some(project))
        }

        async fn save(&self, _project: &Project) -> Result<(), PortError> {
            Err(PortError::Io {
                message: "Save failed".into(),
            })
        }

        async fn create(&self, _project: Project) -> Result<Project, PortError> {
            unimplemented!()
        }

        async fn list(&self) -> Result<Vec<Project>, PortError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
            unimplemented!()
        }
    }

    #[derive(Clone)]
    struct DraftProjectRepo;

    #[async_trait]
    impl ProjectRepository for DraftProjectRepo {
        async fn get(&self, _id: &ProjectId) -> Result<Option<Project>, PortError> {
            // Project is in Draft state, which will fail mark_completed()
            Ok(Some(Project::new("Draft".into())))
        }

        async fn save(&self, _project: &Project) -> Result<(), PortError> {
            Ok(())
        }

        async fn create(&self, _project: Project) -> Result<Project, PortError> {
            unimplemented!()
        }

        async fn list(&self) -> Result<Vec<Project>, PortError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_transition_failure_propagates() {
        let repo = DraftProjectRepo;
        let use_case = HandleJobCompletedUseCase::new(repo);

        let req = HandleJobCompletedRequest {
            job_id: "job-1".into(),
            project_id: ProjectId::new().to_string(),
            is_success: true,
        };

        let result = use_case.execute(req).await;
        assert!(
            result.is_err(),
            "Expected transition to fail for Draft project"
        );
        if let Err(ApplicationError::Domain(e)) = result {
            match e {
                domain::error::DomainError::InvalidStateTransition { .. } => {}
                _ => panic!("Expected InvalidStateTransition error"),
            }
        } else {
            panic!("Expected Domain error for transition failure");
        }
    }

    #[tokio::test]
    async fn test_save_failure_propagates() {
        let repo = FailingSaveRepo;
        let use_case = HandleJobCompletedUseCase::new(repo);

        let req = HandleJobCompletedRequest {
            job_id: "job-1".into(),
            project_id: ProjectId::new().to_string(),
            is_success: true,
        };

        let result = use_case.execute(req).await;
        assert!(result.is_err(), "Expected save to fail");
        if let Err(ApplicationError::Port(e)) = result {
            match e {
                PortError::Io { .. } => {}
                _ => panic!("Expected Io Error"),
            }
        } else {
            panic!("Expected Port error for save failure");
        }
    }
}
