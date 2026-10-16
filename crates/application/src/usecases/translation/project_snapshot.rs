use domain::project::ProjectId;
use ports::translation_links::TranslationLink;

use super::TranslationRunStatus;

pub struct ProjectTranslationSnapshot {
    pub project_id: ProjectId,
    pub entries: Vec<ProjectTranslationSnapshotEntry>,
}

pub struct ProjectTranslationSnapshotEntry {
    pub link: TranslationLink,
    pub run: Option<TranslationRunStatus>,
}
