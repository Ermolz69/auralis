use super::{LocalMediaProjects, db, files, invalid};
use domain::{
    library::{LibraryAsset, LibraryAssetKind, LibraryState},
    media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState},
    outbox::{OutboxMessage, OutboxPayload},
    project::ProjectId,
};
use ports::{artifact_index::ArtifactIndex, error::PortError, media_project::MediaProjectStore};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl LocalMediaProjects {
    pub(super) async fn attach_locked(
        &self,
        op: &str,
        project: &str,
        source_id: &str,
        duplicate: bool,
    ) -> Result<LibraryState, PortError> {
        files::identity(op)?;
        files::identity(project)?;
        self.idle(project).await?;
        let state = self.state().await?;
        if let Some(existing) = self.operation(op).await? {
            let original: String =
                sqlx::query_scalar("SELECT source_asset_id FROM media_copy_operations WHERE id=?")
                    .bind(op)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(db)?;
            if existing.project_id != project || original != source_id {
                return Err(invalid("ID операции уже принадлежит другому добавлению"));
            }
            self.finish_copies().await?;
            return self.state().await;
        }
        let source = state
            .assets
            .iter()
            .find(|a| a.id == source_id)
            .ok_or_else(|| invalid("Материал не найден"))?
            .clone();
        if source.owner_project_id == project && !duplicate {
            return self.attach_existing(project, source_id, state).await;
        }
        let (path, kind) = self.source_file(&source).await?;
        let (size, hash) = if let Some(path) = &path {
            files::digest(path).await?
        } else {
            let bytes = source.text.as_deref().unwrap_or("").as_bytes();
            (bytes.len() as u64, format!("{:x}", Sha256::digest(bytes)))
        };
        if !duplicate {
            let previous: Option<String>=sqlx::query_scalar("SELECT id FROM media_copy_operations WHERE project_id=? AND source_asset_id=? AND source_sha256=? AND state IN ('pending','ready') ORDER BY rowid LIMIT 1")
                .bind(project).bind(source_id).bind(&hash).fetch_optional(&self.pool).await.map_err(db)?;
            if let Some(previous) = previous {
                self.finish_copies().await?;
                if let Some(existing) = self.operation(&previous).await?
                    && existing.status == "ready"
                {
                    self.attach_existing(project, &existing.asset_id, self.state().await?)
                        .await?;
                }
                sqlx::query("INSERT INTO media_copy_operations(id,project_id,source_asset_id,source_sha256,asset_json,staging_key,final_key,state,completed_bytes,total_bytes,error) SELECT ?,project_id,source_asset_id,source_sha256,asset_json,staging_key,final_key,state,completed_bytes,total_bytes,error FROM media_copy_operations WHERE id=?")
                    .bind(op).bind(previous).execute(&self.pool).await.map_err(db)?;
                return self.state().await;
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let ext = path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|s| s.to_str())
            .unwrap_or("txt")
            .to_lowercase();
        if !ext.chars().all(|c| c.is_ascii_alphanumeric()) || ext.len() > 12 {
            return Err(invalid("Неподдерживаемое расширение файла"));
        }
        let staging = format!(".staging/{op}/source.{ext}");
        let final_key = format!(
            "{project}/assets/{}/{id}/source.{ext}",
            files::section(&source.kind)
        );
        let mut asset = source.clone();
        asset.id = id.clone();
        asset.owner_project_id = project.into();
        asset.artifact_id = Some(id.clone());
        asset.parent_id = source.parent_id.clone().filter(|parent| {
            state
                .assets
                .iter()
                .any(|a| &a.id == parent && a.owner_project_id == project)
        });
        asset.opened_at = None;
        if duplicate {
            asset.name = format!("{} — копия", source.name);
        }
        let asset_json = serde_json::to_string(&asset).map_err(invalid)?;
        sqlx::query("INSERT INTO media_copy_operations(id,project_id,source_asset_id,source_sha256,asset_json,staging_key,final_key,state,total_bytes) VALUES(?,?,?,?,?,?,?,'copying',?)")
            .bind(op).bind(project).bind(source_id).bind(&hash).bind(&asset_json).bind(&staging).bind(&final_key).bind(size as i64).execute(&self.pool).await.map_err(db)?;
        let result = self
            .stage_copy(
                op,
                &staging,
                path.as_deref(),
                source.text.as_deref(),
                size,
                &hash,
            )
            .await;
        if let Err(e) = result {
            sqlx::query("UPDATE media_copy_operations SET state=CASE WHEN state='cancelled' THEN state ELSE 'failed' END,error=? WHERE id=?")
                .bind(e.to_string()).bind(op).execute(&self.pool).await.map_err(db)?;
            let _ = tokio::fs::remove_file(files::safe_path(&self.root, &staging)?).await;
            return Err(e);
        }
        let now = chrono::Utc::now();
        let artifact = Artifact {
            id: ArtifactId(uuid::Uuid::parse_str(&id).map_err(invalid)?),
            kind,
            location: ArtifactLocation::StorageKey(final_key.clone()),
            size_bytes: Some(size),
            state: ArtifactState::PendingFinalize,
            created_at: now,
            updated_at: now,
            ready_at: None,
        };
        let project_id = ProjectId(uuid::Uuid::parse_str(project).map_err(invalid)?);
        let mut tx = self.pool.begin().await.map_err(db)?;
        let changed = sqlx::query(
            "UPDATE media_copy_operations SET state='pending' WHERE id=? AND state='copying'",
        )
        .bind(op)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if changed.rows_affected() != 1 {
            return Err(invalid("Копирование отменено"));
        }
        super::super::unit_of_work::insert_new_artifact(&mut tx, &project_id, &artifact).await?;
        super::super::unit_of_work::outbox_writes::save_outbox_message(
            &mut tx,
            &OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
                project_id,
                artifact_id: artifact.id,
                staging_key: staging,
                final_key,
            }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(state)
    }
    async fn attach_existing(
        &self,
        project: &str,
        asset_id: &str,
        mut state: LibraryState,
    ) -> Result<LibraryState, PortError> {
        if !state
            .links
            .iter()
            .any(|l| l.project_id == project && l.asset_id == asset_id)
        {
            state.links.push(domain::library::LibraryLink {
                project_id: project.into(),
                asset_id: asset_id.into(),
                folder_id: None,
            });
            self.publish(project, &state, false).await?;
            let expected = state.revision;
            state.revision += 1;
            let mut tx = self.pool.begin().await.map_err(db)?;
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
                return Err(invalid("Библиотека изменилась; повторите добавление"));
            }
            sqlx::query("UPDATE media_project_receipts SET state='ready' WHERE project_id=?")
                .bind(project)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            tx.commit().await.map_err(db)?;
        }
        Ok(state)
    }
    async fn stage_copy(
        &self,
        op: &str,
        key: &str,
        source: Option<&std::path::Path>,
        text: Option<&str>,
        expected: u64,
        hash: &str,
    ) -> Result<(), PortError> {
        let target = files::safe_path(&self.root, key)?;
        tokio::fs::create_dir_all(
            target
                .parent()
                .ok_or_else(|| invalid("Нет папки staging"))?,
        )
        .await
        .map_err(invalid)?;
        let mut output = tokio::fs::File::create(&target).await.map_err(invalid)?;
        let mut copied = 0u64;
        if let Some(source) = source {
            let mut input = tokio::fs::File::open(source).await.map_err(invalid)?;
            let mut buffer = vec![0; 1024 * 1024];
            loop {
                if self
                    .operation(op)
                    .await?
                    .is_some_and(|o| o.status == "cancelled")
                {
                    return Err(invalid("Копирование отменено"));
                }
                let count = input.read(&mut buffer).await.map_err(invalid)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count]).await.map_err(invalid)?;
                copied += count as u64;
                sqlx::query("UPDATE media_copy_operations SET completed_bytes=? WHERE id=?")
                    .bind(copied as i64)
                    .bind(op)
                    .execute(&self.pool)
                    .await
                    .map_err(db)?;
            }
        } else {
            output
                .write_all(text.unwrap_or("").as_bytes())
                .await
                .map_err(invalid)?;
        }
        output.sync_all().await.map_err(invalid)?;
        drop(output);
        if files::digest(&target).await? != (expected, hash.into()) {
            return Err(invalid("Источник изменился во время копирования"));
        }
        Ok(())
    }
    pub(super) async fn source_file(
        &self,
        asset: &LibraryAsset,
    ) -> Result<(Option<std::path::PathBuf>, ArtifactKind), PortError> {
        if let Some(id) = &asset.artifact_id {
            let published = super::super::SqliteArtifactIndex::new(self.pool.clone())
                .get(&ArtifactId(uuid::Uuid::parse_str(id).map_err(invalid)?))
                .await?;
            // Internal publication may read its finalized staging output; public queries keep it hidden.
            let artifact = if let Some(artifact) = published {
                artifact
            } else {
                let row = sqlx::query_as::<_, super::super::artifact_index::row::ArtifactRow>(
                    "SELECT a.* FROM artifacts a WHERE a.id=? AND a.project_id=? AND a.state='ready' AND EXISTS(SELECT 1 FROM media_copy_operations c WHERE c.state='pending' AND json_extract(c.asset_json,'$.id')=a.id)"
                ).bind(id).bind(&asset.owner_project_id).fetch_optional(&self.pool).await.map_err(db)?
                    .ok_or_else(|| invalid("Файл материала отсутствует"))?;
                super::super::artifact_index::mapper::row_to_artifact(row)?
            };
            if artifact.state != ArtifactState::Ready {
                return Err(invalid("Материал ещё не опубликован"));
            }
            let ArtifactLocation::StorageKey(key) = &artifact.location else {
                return Err(invalid("Нужен управляемый файл"));
            };
            let kind = if asset.kind == LibraryAssetKind::Audio
                && artifact.kind == ArtifactKind::SourceVideo
            {
                ArtifactKind::ExtractedAudio
            } else {
                artifact.kind
            };
            return Ok((Some(files::safe_path(&self.root, key)?), kind));
        }
        if asset.kind != LibraryAssetKind::Text {
            return Err(invalid("Сначала загрузите медиафайл источника"));
        }
        Ok((None, ArtifactKind::GeneratedTranscript))
    }
}
