use domain::project::Project;
use ports::error::PortError;

use super::database::InMemoryDatabase;

pub(super) fn update_project(
    db: &mut InMemoryDatabase,
    project: &Project,
) -> Result<(), PortError> {
    let existing = db
        .projects
        .get(project.id())
        .ok_or_else(|| PortError::NotFound {
            resource: "Project".to_string(),
        })?;
    if existing.revision() != project.revision() {
        return Err(PortError::Conflict {
            resource: "Project".to_string(),
            message: "Project revision changed concurrently".to_string(),
        });
    }
    let mut project = project.clone();
    project
        .advance_revision()
        .map_err(|error| PortError::Conflict {
            resource: "Project".to_string(),
            message: error.to_string(),
        })?;
    db.projects.insert(project.id().clone(), project);
    Ok(())
}
