use domain::project::ProjectId;
use ports::translation_inspection::TranslationSourceFormat;
use std::path::PathBuf;

pub struct ImportTranslationSourceRequest {
    pub project_id: ProjectId,
    pub path: PathBuf,
    pub format: TranslationSourceFormat,
}
