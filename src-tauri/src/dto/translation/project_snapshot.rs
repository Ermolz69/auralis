use application::usecases::translation::ProjectTranslationSnapshot;
use serde::Serialize;

use super::{TranslationLinkDto, TranslationRunSnapshotDto};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTranslationSnapshotDto {
    pub project_id: String,
    pub entries: Vec<ProjectTranslationSnapshotEntryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTranslationSnapshotEntryDto {
    pub link: TranslationLinkDto,
    pub run: Option<TranslationRunSnapshotDto>,
}

impl From<ProjectTranslationSnapshot> for ProjectTranslationSnapshotDto {
    fn from(snapshot: ProjectTranslationSnapshot) -> Self {
        Self {
            project_id: snapshot.project_id.to_string(),
            entries: snapshot
                .entries
                .into_iter()
                .map(|entry| ProjectTranslationSnapshotEntryDto {
                    link: entry.link.into(),
                    run: entry.run.map(Into::into),
                })
                .collect(),
        }
    }
}
