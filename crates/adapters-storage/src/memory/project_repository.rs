use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::repository::ProjectRepository;

#[derive(Clone)]
pub struct InMemoryProjectRepository {
    pub projects: Arc<Mutex<HashMap<ProjectId, Project>>>,
}

impl InMemoryProjectRepository {
    pub fn new() -> Self {
        Self {
            projects: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryProjectRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProjectRepository for InMemoryProjectRepository {
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        let mut lock = self.projects.lock().unwrap();
        lock.insert(project.id().clone(), project.clone());
        Ok(project)
    }

    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        let lock = self.projects.lock().unwrap();
        Ok(lock.get(id).cloned())
    }

    async fn save(&self, project: &Project) -> Result<(), PortError> {
        let mut lock = self.projects.lock().unwrap();
        lock.insert(project.id().clone(), project.clone());
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        let lock = self.projects.lock().unwrap();
        Ok(lock.values().cloned().collect())
    }

    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        let mut lock = self.projects.lock().unwrap();
        lock.remove(id);
        Ok(())
    }
}
