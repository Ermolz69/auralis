use super::LibraryState;
use crate::error::DomainError;
use std::collections::HashSet;

fn invalid() -> DomainError {
    DomainError::ValidationError("Invalid library organization or workspace".into())
}
fn identifier(value: &str) -> Result<(), DomainError> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| invalid())
}

impl LibraryState {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.revision > i64::MAX as u64
            || self.assets.len() > 20_000
            || self.links.len() > 50_000
            || self.folders.len() > 10_000
            || self.workspaces.len() > 100
        {
            return Err(invalid());
        }
        let mut ids = HashSet::new();
        for asset in &self.assets {
            identifier(&asset.id)?;
            identifier(&asset.owner_project_id)?;
            if !ids.insert(&asset.id)
                || asset.name.trim().is_empty()
                || asset.name.len() > 512
                || asset.text.as_ref().is_some_and(|v| v.len() > 1_000_000)
                || asset.tags.len() > 100
                || asset.tags.iter().any(|v| v.len() > 200)
            {
                return Err(invalid());
            }
            if let Some(id) = &asset.artifact_id {
                identifier(id)?;
            }
            if let Some(id) = &asset.parent_id {
                identifier(id)?;
            }
            if asset
                .format
                .as_deref()
                .is_some_and(|v| !["srt", "vtt", "txt", "wav"].contains(&v))
            {
                return Err(invalid());
            }
        }
        let mut folders = HashSet::new();
        for asset in &self.assets {
            let mut visited = HashSet::from([asset.id.as_str()]);
            let mut parent = asset.parent_id.as_deref();
            while let Some(id) = parent {
                if !visited.insert(id) {
                    return Err(invalid());
                }
                let item = self
                    .assets
                    .iter()
                    .find(|a| a.id == id)
                    .ok_or_else(invalid)?;
                parent = item.parent_id.as_deref();
            }
        }
        for folder in &self.folders {
            identifier(&folder.id)?;
            identifier(&folder.project_id)?;
            if !folders.insert(&folder.id)
                || folder.name.trim().is_empty()
                || folder.name.len() > 200
            {
                return Err(invalid());
            }
            let mut visited = HashSet::from([folder.id.as_str()]);
            let mut parent = folder.parent_id.as_deref();
            while let Some(id) = parent {
                if !visited.insert(id) {
                    return Err(invalid());
                }
                let item = self
                    .folders
                    .iter()
                    .find(|f| f.id == id && f.project_id == folder.project_id)
                    .ok_or_else(invalid)?;
                parent = item.parent_id.as_deref();
            }
        }
        let mut links = HashSet::new();
        for link in &self.links {
            identifier(&link.project_id)?;
            if !ids.contains(&link.asset_id)
                || !links.insert((&link.project_id, &link.asset_id))
                || link.folder_id.as_ref().is_some_and(|id| {
                    !self
                        .folders
                        .iter()
                        .any(|f| &f.id == id && f.project_id == link.project_id)
                })
            {
                return Err(invalid());
            }
        }
        for id in self
            .standalone_owners
            .iter()
            .chain(&self.removed_projects)
            .chain(&self.hidden_assets)
        {
            identifier(id)?;
        }
        let mut workspaces = HashSet::new();
        for workspace in &self.workspaces {
            if workspace.id.len() > 100
                || !workspaces.insert(&workspace.id)
                || workspace.tabs.len() > 100
                || workspace.project_id.is_none() == workspace.asset_id.is_none()
            {
                return Err(invalid());
            }
            if let Some(id) = &workspace.project_id {
                identifier(id)?;
            }
            if workspace
                .asset_id
                .as_ref()
                .is_some_and(|id| !ids.contains(id))
            {
                return Err(invalid());
            }
            let mut tabs = HashSet::new();
            for tab in &workspace.tabs {
                identifier(&tab.id)?;
                if !tabs.insert(&tab.id)
                    || tab.draft.len() > 1_100_000
                    || tab.asset_id.as_ref().is_some_and(|id| !ids.contains(id))
                {
                    return Err(invalid());
                }
            }
            if workspace
                .active_tab
                .as_ref()
                .is_some_and(|id| !tabs.contains(id))
            {
                return Err(invalid());
            }
        }
        if self.browser.search.len() > 1000
            || !["", "all", "unassigned", "recent", "project"]
                .contains(&self.browser.scope.as_str())
            || !["", "text", "subtitle", "audio", "video"].contains(&self.browser.kind.as_str())
        {
            return Err(invalid());
        }
        Ok(())
    }
}
