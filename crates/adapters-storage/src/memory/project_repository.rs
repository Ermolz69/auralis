use async_trait::async_trait;

use std::sync::{Arc, Mutex};

use domain::project::{Project, ProjectId};
use ports::error::PortError;
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
}

#[async_trait]
impl ProjectRepository for InMemoryProjectRepository {
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        let mut lock = self.db.lock().unwrap();
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
        let lock = self.db.lock().unwrap();
        Ok(lock.projects.get(id).cloned())
    }

    async fn save(&self, project: &Project) -> Result<(), PortError> {
        let mut lock = self.db.lock().unwrap();
        if !lock.projects.contains_key(project.id()) {
            return Err(PortError::NotFound {
                resource: "Project".to_string(),
            });
        }
        lock.projects.insert(project.id().clone(), project.clone());
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        let lock = self.db.lock().unwrap();
        Ok(lock.projects.values().cloned().collect())
    }

    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        let mut lock = self.db.lock().unwrap();
        lock.projects.remove(id);
        Ok(())
    }
}
