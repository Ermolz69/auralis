use super::hydrate::{asset_kind, hydrate_project};
use crate::error::ApplicationError;
use domain::{library::LibraryState, media::ArtifactState};
use ports::{
    artifact_index::ArtifactIndex, error::PortError, library::LibraryRepository,
    repository::ProjectRepository,
};
use std::{collections::HashSet, sync::Arc};

pub struct LibraryUseCase {
    repository: Arc<dyn LibraryRepository>,
    projects: Arc<dyn ProjectRepository>,
    artifacts: Arc<dyn ArtifactIndex>,
    text_reader: Option<Arc<dyn ports::library::LibraryTextReader>>,
}
impl LibraryUseCase {
    pub fn new(
        repository: Arc<dyn LibraryRepository>,
        projects: Arc<dyn ProjectRepository>,
        artifacts: Arc<dyn ArtifactIndex>,
    ) -> Self {
        Self {
            repository,
            projects,
            artifacts,
            text_reader: None,
        }
    }
    pub fn with_text_reader(mut self, reader: Arc<dyn ports::library::LibraryTextReader>) -> Self {
        self.text_reader = Some(reader);
        self
    }
    pub async fn read_text(&self, path: &str) -> Result<String, ApplicationError> {
        let reader = self
            .text_reader
            .as_ref()
            .ok_or_else(|| PortError::Unsupported {
                message: "Text import is unavailable".into(),
            })?;
        Ok(reader.read_text(path).await?)
    }

    pub async fn load(&self) -> Result<LibraryState, ApplicationError> {
        let mut state = self.repository.load().await?;
        let projects = self.projects.list().await?;
        let ids: HashSet<_> = projects.iter().map(|p| p.id().to_string()).collect();
        state.standalone_owners.retain(|id| ids.contains(id));
        state.removed_projects.retain(|id| ids.contains(id));
        state.assets.retain(|a| ids.contains(&a.owner_project_id));
        state.folders.retain(|f| ids.contains(&f.project_id));
        for project in projects {
            let artifacts = self.artifacts.list_by_project(project.id()).await?;
            state.assets.retain(|a| {
                a.owner_project_id != project.id().to_string()
                    || a.artifact_id.as_ref().is_none_or(|id| {
                        artifacts.iter().any(|item| {
                            item.id.to_string() == *id && item.state == ArtifactState::Ready
                        })
                    })
            });
            hydrate_project(&mut state, &project, &artifacts);
        }
        let assets: HashSet<_> = state.assets.iter().map(|a| a.id.clone()).collect();
        for asset in &mut state.assets {
            if asset
                .parent_id
                .as_ref()
                .is_some_and(|id| !assets.contains(id))
            {
                asset.parent_id = None;
            }
        }
        let folders: HashSet<_> = state.folders.iter().map(|f| f.id.clone()).collect();
        state
            .links
            .retain(|l| assets.contains(&l.asset_id) && ids.contains(&l.project_id));
        for link in &mut state.links {
            if link
                .folder_id
                .as_ref()
                .is_some_and(|id| !folders.contains(id))
            {
                link.folder_id = None;
            }
        }
        state.workspaces.retain(|w| {
            w.project_id.as_ref().is_some_and(|id| ids.contains(id))
                || w.asset_id.as_ref().is_some_and(|id| assets.contains(id))
        });
        for workspace in &mut state.workspaces {
            workspace
                .tabs
                .retain(|tab| tab.asset_id.as_ref().is_none_or(|id| assets.contains(id)));
            if workspace
                .active_tab
                .as_ref()
                .is_some_and(|id| !workspace.tabs.iter().any(|t| &t.id == id))
            {
                workspace.active_tab = workspace.tabs.first().map(|t| t.id.clone());
            }
        }
        Ok(state)
    }

    pub async fn save(&self, state: LibraryState) -> Result<LibraryState, ApplicationError> {
        state.validate()?;
        let projects = self.projects.list().await?;
        let ids: HashSet<_> = projects.iter().map(|p| p.id().to_string()).collect();
        let invalid = || PortError::InvalidSource {
            message: "Library refers to a missing project or an unready asset".into(),
        };
        for id in state
            .standalone_owners
            .iter()
            .chain(&state.removed_projects)
            .chain(state.links.iter().map(|l| &l.project_id))
            .chain(state.folders.iter().map(|f| &f.project_id))
            .chain(
                state
                    .workspaces
                    .iter()
                    .filter_map(|w| w.project_id.as_ref()),
            )
        {
            if !ids.contains(id) {
                return Err(invalid().into());
            }
        }
        for asset in &state.assets {
            if !ids.contains(&asset.owner_project_id) {
                return Err(invalid().into());
            }
            if let Some(id) = &asset.artifact_id {
                let owner = projects
                    .iter()
                    .find(|p| p.id().to_string() == asset.owner_project_id)
                    .ok_or_else(invalid)?;
                let artifacts = self.artifacts.list_by_project(owner.id()).await?;
                if !artifacts.iter().any(|a| {
                    a.id.to_string() == *id
                        && a.state == ArtifactState::Ready
                        && asset_kind(&a.kind, owner).as_ref() == Some(&asset.kind)
                }) {
                    return Err(invalid().into());
                }
            } else if asset.kind != domain::library::LibraryAssetKind::Text
                && !projects.iter().any(|p| {
                    p.id().to_string() == asset.owner_project_id
                        && p.source().is_some()
                        && asset.id == asset.owner_project_id
                })
            {
                return Err(invalid().into());
            }
        }
        Ok(self.repository.save(state).await?)
    }
}
