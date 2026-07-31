use std::collections::HashMap;

use domain::job::{Job, JobId};
use domain::project::{Project, ProjectId};

#[derive(Default)]
pub struct InMemoryDatabase {
    pub projects: HashMap<ProjectId, Project>,
    pub jobs: HashMap<JobId, Job>,
}

impl InMemoryDatabase {
    pub fn new() -> Self {
        Self {
            projects: HashMap::new(),
            jobs: HashMap::new(),
        }
    }
}
