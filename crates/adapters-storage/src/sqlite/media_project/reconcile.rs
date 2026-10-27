use super::{LocalMediaProjects, db, files, invalid};
use domain::library::{LibraryAsset, LibraryLink, LibraryState};
use ports::{error::PortError, library::LibraryRepository};
use sqlx::Row;

impl LocalMediaProjects {
    pub(super) async fn reconcile_locked(
        &self,
        mut state: LibraryState,
    ) -> Result<LibraryState, PortError> {
        // Interrupted copies have no committed finalization intent. Retrying is explicit.
        sqlx::query("UPDATE media_copy_operations SET state='failed',error='Копирование прервано. Повторите добавление.' WHERE state='copying'")
            .execute(&self.pool).await.map_err(db)?;
        self.finish_copies().await?;
        let stored = self.state().await?;
        if stored.revision != state.revision {
            state = stored;
        }
        let unpublished:Vec<String>=sqlx::query_scalar("SELECT json_extract(o.asset_json,'$.id') FROM media_copy_operations o WHERE state!='ready' AND NOT EXISTS(SELECT 1 FROM media_copy_operations r WHERE r.state='ready' AND json_extract(r.asset_json,'$.id')=json_extract(o.asset_json,'$.id'))")
            .fetch_all(&self.pool).await.map_err(db)?;
        state.assets.retain(|a| !unpublished.contains(&a.id));
        state.links.retain(|l| !unpublished.contains(&l.asset_id));
        // Save hydration before migrating, so the durable journal can resolve source identities.
        let old = self.state().await?;
        if serde_json::to_value(&old).map_err(invalid)?
            != serde_json::to_value(&state).map_err(invalid)?
        {
            state =
                super::super::library_repository::SqliteLibraryRepository::new(self.pool.clone())
                    .save(state)
                    .await?;
        }
        let projects: Vec<String> = sqlx::query_scalar("SELECT id FROM projects")
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;
        for project in projects {
            if state.removed_projects.contains(&project) || self.idle(&project).await.is_err() {
                continue;
            }
            let foreign: Vec<String> = state
                .links
                .iter()
                .filter(|l| l.project_id == project)
                .filter_map(|l| {
                    state
                        .assets
                        .iter()
                        .find(|a| a.id == l.asset_id && a.owner_project_id != project)
                        .map(|a| a.id.clone())
                })
                .collect();
            if !foreign.is_empty() {
                self.backup(&project, &state).await?;
                for asset in foreign {
                    // No media content is inferred from a URL placeholder.
                    if state
                        .assets
                        .iter()
                        .any(|a| a.id == asset && a.artifact_id.is_none() && a.text.is_none())
                    {
                        continue;
                    }
                    if let Err(error) = self
                        .attach_locked(&uuid::Uuid::new_v4().to_string(), &project, &asset, false)
                        .await
                    {
                        tracing::warn!(project_id=%project,error=%error,"Media migration deferred");
                    }
                }
                continue;
            }
            if let Err(error) = self.publish(&project, &state, true).await {
                // A failed project migration must not hide the rest of the existing library.
                tracing::warn!(project_id=%project,error=%error,"Media project needs attention");
            }
        }
        self.state().await
    }
    pub(super) async fn finish_copies(&self) -> Result<(), PortError> {
        let rows=sqlx::query("SELECT id,project_id,source_asset_id,source_sha256,asset_json,final_key FROM media_copy_operations WHERE state='pending' ORDER BY rowid")
            .fetch_all(&self.pool).await.map_err(db)?;
        for row in rows {
            let op: String = row.try_get("id").map_err(db)?;
            let project: String = row.try_get("project_id").map_err(db)?;
            let source: String = row.try_get("source_asset_id").map_err(db)?;
            let asset: LibraryAsset =
                serde_json::from_str(&row.try_get::<String, _>("asset_json").map_err(db)?)
                    .map_err(invalid)?;
            let ready: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=? AND state='ready')",
            )
            .bind(&asset.id)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
            if !ready || self.idle(&project).await.is_err() {
                continue;
            }
            let path = files::safe_path(
                &self.root,
                &row.try_get::<String, _>("final_key").map_err(db)?,
            )?;
            let expected: String = row.try_get("source_sha256").map_err(db)?;
            let verification = files::digest(&path).await;
            let error = match verification {
                Ok((_, hash)) if hash == expected => None,
                Ok(_) => Some("Контрольная сумма копии не совпадает".to_string()),
                Err(error) => Some(error.to_string()),
            };
            if let Some(error) = error {
                sqlx::query("UPDATE media_copy_operations SET state='failed',error=? WHERE id=? AND state='pending'")
                    .bind(error).bind(&op).execute(&self.pool).await.map_err(db)?;
                continue;
            }
            let mut state = self.state().await?;
            self.backup(&project, &state).await?;
            let migrating = state
                .assets
                .iter()
                .any(|a| a.id == source && a.owner_project_id != project)
                && state
                    .links
                    .iter()
                    .any(|l| l.asset_id == source && l.project_id == project);
            let previous_folder = state
                .links
                .iter()
                .find(|l| l.project_id == project && l.asset_id == source)
                .and_then(|l| l.folder_id.clone());
            if !state.assets.iter().any(|a| a.id == asset.id) {
                state.assets.push(asset.clone());
            }
            let mappings:Vec<(String,String)>=sqlx::query_as("SELECT source_asset_id,json_extract(asset_json,'$.id') FROM media_copy_operations WHERE project_id=? AND state IN ('pending','ready')").bind(&project).fetch_all(&self.pool).await.map_err(db)?;
            let originals = state.assets.clone();
            for item in state
                .assets
                .iter_mut()
                .filter(|a| a.owner_project_id == project)
            {
                let original = mappings
                    .iter()
                    .find(|(_, copy)| copy == &item.id)
                    .and_then(|(source, _)| originals.iter().find(|a| &a.id == source));
                if let Some(parent) = original
                    .and_then(|a| a.parent_id.as_ref())
                    .and_then(|parent| mappings.iter().find(|(source, _)| source == parent))
                    .map(|(_, copy)| copy.clone())
                    && originals.iter().any(|a| a.id == parent)
                {
                    item.parent_id = Some(parent);
                }
            }
            state.links.retain(|l| {
                !(l.project_id == project
                    && ((migrating && l.asset_id == source) || l.asset_id == asset.id))
            });
            state.links.push(LibraryLink {
                asset_id: asset.id.clone(),
                project_id: project.clone(),
                folder_id: previous_folder,
            });
            for workspace in state
                .workspaces
                .iter_mut()
                .filter(|w| w.project_id.as_deref() == Some(&project))
            {
                for tab in &mut workspace.tabs {
                    if migrating && tab.asset_id.as_deref() == Some(&source) {
                        tab.asset_id = Some(asset.id.clone());
                    }
                }
            }
            if migrating
                && state.browser.project_id.as_deref() == Some(&project)
                && state.browser.selected_id.as_deref() == Some(&source)
            {
                state.browser.selected_id = Some(asset.id.clone());
            }
            // Files and CONTRACT are checked before the new membership becomes visible.
            if let Err(error) = self.publish(&project, &state, false).await {
                sqlx::query("UPDATE media_copy_operations SET error=? WHERE id=?")
                    .bind(error.to_string())
                    .bind(&op)
                    .execute(&self.pool)
                    .await
                    .map_err(db)?;
                continue;
            }
            let mut tx = self.pool.begin().await.map_err(db)?;
            let expected = state.revision;
            state.revision += 1;
            let changed = sqlx::query(
                "UPDATE workspace_library SET payload=?,revision=? WHERE id=1 AND revision=?",
            )
            .bind(serde_json::to_string(&state).map_err(invalid)?)
            .bind(state.revision as i64)
            .bind(expected as i64)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if changed.rows_affected() != 1 {
                return Err(ports::error::PortError::Conflict {
                    resource: "Library".into(),
                    message: "Библиотека изменилась во время копирования".into(),
                });
            }
            let changed=sqlx::query("UPDATE media_copy_operations SET state='ready',error=NULL,completed_bytes=total_bytes WHERE id=? AND state='pending'").bind(op).execute(&mut *tx).await.map_err(db)?;
            if changed.rows_affected() != 1 {
                tx.rollback().await.map_err(db)?;
                self.publish(&project, &self.state().await?, true).await?;
                continue;
            }
            sqlx::query("UPDATE media_project_receipts SET state='ready' WHERE project_id=?")
                .bind(&project)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            tx.commit().await.map_err(db)?;
        }
        Ok(())
    }
    pub(super) async fn backup(&self, id: &str, state: &LibraryState) -> Result<(), PortError> {
        let path = files::safe_path(
            &self.root,
            &format!("{id}/work/migration/library-before.json"),
        )?;
        if !path.exists() {
            files::atomic(&path, &serde_json::to_vec_pretty(state).map_err(invalid)?).await?;
        }
        let metadata = files::safe_path(
            &self.root,
            &format!("{id}/work/migration/project-before.json"),
        )?;
        if !metadata.exists() {
            let snapshot: String = sqlx::query_scalar("SELECT json_object('id',p.id,'title',p.title,'revision',p.revision,'source',p.source_json,'metadata',p.metadata_json,'contract',c.snapshot_json) FROM projects p LEFT JOIN puza_contract_projects c ON c.project_id=p.id WHERE p.id=?")
                .bind(id).fetch_one(&self.pool).await.map_err(db)?;
            files::atomic(&metadata, snapshot.as_bytes()).await?;
        }
        Ok(())
    }
}
