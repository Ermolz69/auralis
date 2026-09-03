use async_trait::async_trait;

use std::sync::{Arc, Mutex};

use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::project_update::ProjectUpdate;
use ports::repository::ProjectRepository;

use super::database::InMemoryDatabase;

#[derive(Clone)]
pub struct InMemoryProjectRepository {
    pub db: Arc<Mutex<InMemoryDatabase>>,
}

impl InMemoryProjectRepository {
    pub fn new(db: Arc<Mutex<InMemoryDatabase>>) -> Self {
        Self { db }
    }

    fn lock_db(&self) -> Result<std::sync::MutexGuard<'_, InMemoryDatabase>, PortError> {
        self.db.lock().map_err(|_| PortError::Storage {
            operation: "lock_in_memory_db",
            message: "Mutex poisoned".to_string(),
        })
    }
}

#[async_trait]
impl ProjectRepository for InMemoryProjectRepository {
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        let mut lock = self.lock_db()?;
        if lock.projects.contains_key(project.id()) {
            return Err(PortError::Conflict {
                resource: "Project".to_string(),
                message: format!("Project with id {} already exists", project.id()),
            });
        }
        lock.projects.insert(project.id().clone(), project.clone());
        Ok(project)
    }

    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        let lock = self.lock_db()?;
        Ok(lock.projects.get(id).cloned())
    }

    async fn update(
        &self,
        id: &ProjectId,
        expected_revision: u64,
        update: ProjectUpdate,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Project, PortError> {
        let mut lock = self.lock_db()?;
        let mut project = lock
            .projects
            .get(id)
            .cloned()
            .ok_or_else(|| PortError::NotFound {
                resource: "Project".to_string(),
            })?;
        if project.revision() != expected_revision {
            return Err(PortError::Conflict {
                resource: "Project".to_string(),
                message: "Project revision changed concurrently".to_string(),
            });
        }
        let conflict = |error: domain::error::DomainError| PortError::Conflict {
            resource: "Project".to_string(),
            message: error.to_string(),
        };
        match update {
            ProjectUpdate::Rename { title } => project.set_title(title),
            ProjectUpdate::ImportSource { source, metadata } => {
                project.import_source(source, metadata.map(|value| *value))
            }
            ProjectUpdate::MarkReadyForProcessing => project.mark_ready_for_processing(),
        }
        .map_err(conflict)?;
        project.advance_revision().map_err(conflict)?;
        let mut snapshot = project.to_snapshot();
        snapshot.updated_at = updated_at;
        let project = Project::from_snapshot(snapshot).map_err(conflict)?;
        lock.projects.insert(project.id().clone(), project.clone());
        Ok(project)
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        let lock = self.lock_db()?;
        Ok(lock.projects.values().cloned().collect())
    }

    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        let mut lock = self.lock_db()?;
        lock.projects.remove(id);
        Ok(())
    }
}
