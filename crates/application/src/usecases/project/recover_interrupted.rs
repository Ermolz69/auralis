use domain::project::ProjectStatus;
use ports::repository::ProjectRepository;
use std::sync::Arc;

pub struct RecoverInterruptedProjectsUseCase {
    project_repo: Arc<dyn ProjectRepository>,
}

impl RecoverInterruptedProjectsUseCase {
    pub fn new(project_repo: Arc<dyn ProjectRepository>) -> Self {
        Self { project_repo }
    }

    pub async fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        let projects = self.project_repo.list().await?;

        for mut project in projects {
            if *project.status() == ProjectStatus::Processing {
                // Interrupted by application restart, failing it
                if let Err(e) = project.mark_failed() {
                    println!(
                        "Failed to mark interrupted project {} as failed: {}",
                        project.id(),
                        e
                    );
                    continue;
                }

                if let Err(e) = self.project_repo.save(&project).await {
                    println!("Failed to save interrupted project {}: {}", project.id(), e);
                } else {
                    println!(
                        "Recovered interrupted project {} by marking it as failed (restart)",
                        project.id()
                    );
                }
            }
        }

        Ok(())
    }
}
