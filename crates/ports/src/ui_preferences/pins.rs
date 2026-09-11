use domain::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPin {
    pub project_id: ProjectId,
    pub pinned: bool,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPins {
    pub entries: Vec<ProjectPin>,
    pub migrated: bool,
}
