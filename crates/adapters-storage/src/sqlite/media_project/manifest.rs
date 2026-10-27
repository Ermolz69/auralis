use super::{LocalMediaProjects, db, files, invalid};
use domain::library::LibraryState;
use ports::error::PortError;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

impl LocalMediaProjects {
    pub(super) async fn publish(
        &self,
        id: &str,
        state: &LibraryState,
        ready: bool,
    ) -> Result<(), PortError> {
        files::identity(id)?;
        let root = files::safe_path(&self.root, id)?;
        for section in [
            "assets/video",
            "assets/audio",
            "assets/text",
            "assets/subtitles",
            "results",
            "puza/input",
            "puza/documents",
            "puza/work",
            "work",
        ] {
            tokio::fs::create_dir_all(files::safe_path(&root, section)?)
                .await
                .map_err(invalid)?;
        }
        let snapshot: Option<String> = sqlx::query_scalar(
            "SELECT snapshot_json FROM puza_contract_projects WHERE project_id=?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        let snapshot: ports::puza_contract::PuzaContractSnapshot = serde_json::from_str(
            &snapshot.ok_or_else(|| invalid("Проект не прошёл проверку CONTRACT"))?,
        )
        .map_err(invalid)?;
        let cast: Value = serde_json::from_str(&snapshot.cast_json).map_err(invalid)?;
        let voices: Value = serde_json::from_str(&snapshot.bindings_json).map_err(invalid)?;
        let publications: String = sqlx::query_scalar("SELECT COALESCE(group_concat(publication_json),'') FROM speech_publications p JOIN speech_run_intents i ON i.run_id=p.run_id WHERE i.project_id=? AND p.ready=1").bind(id).fetch_one(&self.pool).await.map_err(db)?;
        let translations: String = sqlx::query_scalar("SELECT COALESCE(group_concat(p.artifact_id),'') FROM translation_publications p JOIN project_translations t ON t.translation_id=p.translation_id WHERE t.project_id=? AND p.state='ready'").bind(id).fetch_one(&self.pool).await.map_err(db)?;
        let reviews: String = sqlx::query_scalar(
            "SELECT COALESCE(group_concat(run_id),'') FROM media_run_sources WHERE project_id=?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(db)?;
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&json!([
                    state
                        .assets
                        .iter()
                        .filter(|a| a.owner_project_id == id)
                        .collect::<Vec<_>>(),
                    state
                        .links
                        .iter()
                        .filter(|l| l.project_id == id)
                        .collect::<Vec<_>>(),
                    state.hidden_assets,
                    state.standalone_owners,
                    state.folders,
                    cast,
                    voices,
                    publications,
                    translations,
                    reviews
                ]))
                .map_err(invalid)?
            )
        );
        if self.projection_cache.lock().await.get(id) == Some(&fingerprint) {
            return Ok(());
        }
        let previous: Option<String> = sqlx::query_scalar(
            "SELECT manifest_json FROM media_project_receipts WHERE project_id=?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;
        let previous: Value = previous
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(invalid)?
            .unwrap_or(json!({}));
        let assets = self.manifest_assets(id, state).await?;
        let (documents, inputs) = self.manifest_documents(&root, &assets, &previous).await?;
        let results = self.manifest_results(id, &assets).await?;
        let tracks = self.manifest_tracks(id, state, &assets, &previous).await?;
        let value = json!({"version":1,"project_id":id,"revision":state.revision,"assets":assets,"tracks":tracks,"results":results,"documents":documents,"voices":voices});
        let mut comparable = previous.clone();
        comparable["revision"] = value["revision"].clone();
        if comparable == value {
            let published: bool = sqlx::query_scalar(
                "SELECT state='ready' FROM media_project_receipts WHERE project_id=?",
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;
            if published {
                self.projection_cache
                    .lock()
                    .await
                    .insert(id.into(), fingerprint);
                return Ok(());
            }
        }
        self.projection_cache.lock().await.remove(id);
        self.backup(id, state).await?;
        let bytes = serde_json::to_vec(&value).map_err(invalid)?;
        sqlx::query("INSERT INTO media_project_receipts(project_id,manifest_json,manifest_sha256,state,backup_json) VALUES(?,?,?,'publishing',?) ON CONFLICT(project_id) DO UPDATE SET manifest_json=excluded.manifest_json,manifest_sha256=excluded.manifest_sha256,state='publishing'")
            .bind(id).bind(String::from_utf8(bytes.clone()).map_err(invalid)?).bind(format!("{:x}",Sha256::digest(&bytes)))
            .bind(serde_json::to_string(state).map_err(invalid)?).execute(&self.pool).await.map_err(db)?;
        let project = json!({"version":1,"documents":inputs});
        let project_bytes = serde_json::to_vec(&project).map_err(invalid)?;
        let project_hash = format!("{:x}", Sha256::digest(&project_bytes));
        let manifest_path = files::safe_path(&root, "puza/manifest.json")?;
        let old_manifest: Option<Value> = match tokio::fs::read(&manifest_path).await {
            Ok(bytes) => Some(serde_json::from_slice(&bytes).map_err(invalid)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(invalid(e)),
        };
        let mut entries = serde_json::Map::new();
        for document in &documents {
            let key = document["document_id"]
                .as_str()
                .ok_or_else(|| invalid("Нет ID документа"))?;
            let unchanged = previous["documents"]
                .as_array()
                .is_some_and(|items| items.contains(document));
            let mut entry = old_manifest
                .as_ref()
                .and_then(|m| m["documents"].get(key))
                .cloned()
                .unwrap_or(json!({"status":"pending","published":null}));
            if !unchanged {
                entry["status"] = json!(if entry["published"].is_null() {
                    "pending"
                } else {
                    "stale"
                });
            }
            entries.insert(key.into(), entry);
        }
        let manifest = json!({"version":1,"project_sha256":project_hash,"documents":entries});
        files::atomic(
            &files::safe_path(&root, "puza/project.json")?,
            &project_bytes,
        )
        .await?;
        files::atomic(
            &files::safe_path(&root, "puza/cast.json")?,
            &serde_json::to_vec(&cast).map_err(invalid)?,
        )
        .await?;
        files::atomic(
            &manifest_path,
            &serde_json::to_vec(&manifest).map_err(invalid)?,
        )
        .await?;
        files::atomic(&files::safe_path(&root, "auralis-project.json")?, &bytes).await?;
        self.validator.validate(&root).await?;
        if ready {
            self.projection_cache
                .lock()
                .await
                .insert(id.into(), fingerprint);
            sqlx::query("UPDATE media_project_receipts SET state='ready' WHERE project_id=?")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(db)?;
        }
        Ok(())
    }
}
