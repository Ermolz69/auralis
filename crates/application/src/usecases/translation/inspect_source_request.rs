use domain::media::ArtifactId;
use domain::project::ProjectId;
use ports::translation_inspection::TranslationSourceFormat;

pub struct InspectTranslationSourceRequest {
    pub project_id: ProjectId,
    pub source_artifact_id: ArtifactId,
    pub format: TranslationSourceFormat,
}
