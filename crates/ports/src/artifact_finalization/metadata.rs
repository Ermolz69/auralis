use domain::media::Artifact;

pub enum FinalizationMetadata {
    ProjectDeleted,
    Artifact(Artifact),
}
