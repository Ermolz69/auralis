use domain::media::Artifact;
use domain::project::ProjectId;

pub struct CommitProjectDelete {
    pub project_id: ProjectId,
    pub artifacts: Vec<Artifact>,
}
