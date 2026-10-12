use domain::media::Artifact;

pub enum FinalizationMetadata {
    ProjectDeleted,
    ArtifactDiscarded,
    Artifact(Artifact),
}
