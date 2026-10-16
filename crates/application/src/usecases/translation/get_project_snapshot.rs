use super::{
    GetTranslationRunStatusUseCase, ListProjectTranslationsUseCase, ProjectTranslationSnapshot,
    ProjectTranslationSnapshotEntry, TranslationRunRequest,
};
use crate::error::ApplicationError;
use domain::project::ProjectId;
use futures::future::try_join_all;
use ports::error::PortError;
use ports::translation_links::TranslationLinkStore;
use std::sync::Arc;

pub struct GetProjectTranslationSnapshotUseCase {
    list: ListProjectTranslationsUseCase,
    status: GetTranslationRunStatusUseCase,
    links: Arc<dyn TranslationLinkStore>,
}

impl GetProjectTranslationSnapshotUseCase {
    pub fn new(
        list: ListProjectTranslationsUseCase,
        status: GetTranslationRunStatusUseCase,
        links: Arc<dyn TranslationLinkStore>,
    ) -> Self {
        Self {
            list,
            status,
            links,
        }
    }

    pub async fn execute(
        &self,
        project_id: &ProjectId,
    ) -> Result<ProjectTranslationSnapshot, ApplicationError> {
        for attempt in 0..2 {
            let initial = self.list.execute(project_id).await?;
            let entries: Result<Vec<ProjectTranslationSnapshotEntry>, ApplicationError> =
                try_join_all(initial.iter().map(|link| async {
                    let run = match link.active_run_id {
                        Some(run_id) => {
                            let status = self
                                .status
                                .execute(TranslationRunRequest {
                                    project_id: project_id.clone(),
                                    translation_id: link.translation_id,
                                })
                                .await?;
                            if status.snapshot.run_id != run_id {
                                return Err(PortError::Conflict {
                                    resource: "translation project snapshot".into(),
                                    message: "active run changed during snapshot read".into(),
                                }
                                .into());
                            }
                            Some(status)
                        }
                        None => None,
                    };
                    Ok(ProjectTranslationSnapshotEntry {
                        link: link.clone(),
                        run,
                    })
                }))
                .await;
            let latest = self.links.list_by_project(project_id).await?;
            if latest != initial {
                if attempt == 0 {
                    continue;
                }
                return Err(PortError::Conflict {
                    resource: "translation project snapshot".into(),
                    message: "project links changed during snapshot read".into(),
                }
                .into());
            }
            return Ok(ProjectTranslationSnapshot {
                project_id: project_id.clone(),
                entries: entries?,
            });
        }
        unreachable!()
    }
}
