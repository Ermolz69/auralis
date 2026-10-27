use super::{LocalMediaProjects, db, files, invalid};
use domain::library::LibraryState;
use ports::error::PortError;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::Path;

impl LocalMediaProjects {
    pub(super) async fn manifest_assets(
        &self,
        id: &str,
        state: &LibraryState,
    ) -> Result<Vec<Value>, PortError> {
        let root = files::safe_path(&self.root, id)?;
        let selected: Vec<_> = state
            .assets
            .iter()
            .filter(|a| {
                a.owner_project_id == id
                    && !state.hidden_assets.contains(&a.id)
                    && (state.standalone_owners.contains(&id.to_owned())
                        || state
                            .links
                            .iter()
                            .any(|l| l.project_id == id && l.asset_id == a.id))
            })
            .collect();
        let mut assets = Vec::new();
        for asset in &selected {
            let (path, _) = match self.source_file(asset).await {
                Ok(v) => v,
                Err(_) if asset.artifact_id.is_none() && asset.text.is_none() => {
                    (None, domain::media::ArtifactKind::SourceVideo)
                }
                Err(e) => return Err(e),
            };
            let ext = path
                .as_ref()
                .and_then(|p| p.extension())
                .and_then(|x| x.to_str())
                .unwrap_or("txt")
                .to_lowercase();
            if !ext.chars().all(|c| c.is_ascii_alphanumeric()) || ext.len() > 12 {
                return Err(invalid("Недопустимое расширение материала"));
            }
            let receipt = if let Some(path) = &path {
                Some(files::digest(path).await?)
            } else {
                asset.text.as_ref().map(|text| {
                    (
                        text.len() as u64,
                        format!("{:x}", Sha256::digest(text.as_bytes())),
                    )
                })
            };
            let relative = receipt.as_ref().map(|r| {
                format!(
                    "assets/{}/{}/{}.{}",
                    files::section(&asset.kind),
                    asset.id,
                    r.1,
                    ext
                )
            });
            if let Some(relative) = &relative {
                let target = files::safe_path(&root, relative)?;
                if let Some(path) = &path {
                    files::copy_verified(path, &target).await?;
                } else if !target.exists() {
                    files::atomic(&target, asset.text.as_deref().unwrap_or("").as_bytes()).await?;
                }
                if Some(files::digest(&target).await?) != receipt {
                    return Err(invalid("Изменён файл материала проекта"));
                }
            }
            let origin=sqlx::query("SELECT source_asset_id,source_sha256 FROM media_copy_operations WHERE json_extract(asset_json,'$.id')=? ORDER BY rowid DESC LIMIT 1").bind(&asset.id).fetch_optional(&self.pool).await.map_err(db)?;
            let source_id = origin
                .as_ref()
                .map(|r| r.try_get::<String, _>("source_asset_id").map_err(db))
                .transpose()?;
            let source_hash = origin
                .as_ref()
                .map(|r| r.try_get::<Option<String>, _>("source_sha256").map_err(db))
                .transpose()?
                .flatten();
            let folder = state
                .links
                .iter()
                .find(|l| l.project_id == id && l.asset_id == asset.id)
                .and_then(|l| l.folder_id.as_ref())
                .and_then(|f| state.folders.iter().find(|item| &item.id == f))
                .map(|f| f.name.clone());
            let parent = asset
                .parent_id
                .as_ref()
                .filter(|parent| selected.iter().any(|a| &a.id == *parent));
            let source: Option<String> =
                sqlx::query_scalar("SELECT source_json FROM projects WHERE id=?")
                    .bind(&asset.owner_project_id)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(db)?;
            let source: Value = source
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(Value::Null);
            let url = if asset.artifact_id.is_none() {
                source
                    .get("YoutubeUrl")
                    .or_else(|| source.get("RemoteUrl"))
                    .and_then(|s| s.get("url"))
                    .cloned()
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            assets.push(json!({"id":asset.id,"kind":asset.kind,"name":asset.name,"path":relative,"sha256":receipt.as_ref().map(|r|&r.1),"bytes":receipt.as_ref().map(|r|r.0),"artifact_id":asset.artifact_id,"source_asset_id":source_id,"source_sha256":source_hash,"parent_id":parent,"tags":asset.tags,"legacy_folder":folder,"source_url":url}));
        }
        Ok(assets)
    }
    pub(super) async fn manifest_documents(
        &self,
        root: &Path,
        assets: &[Value],
        previous: &Value,
    ) -> Result<(Vec<Value>, Vec<Value>), PortError> {
        let old = match previous.get("documents") {
            None => Vec::new(),
            Some(Value::Array(documents)) => documents.clone(),
            Some(_) => return Err(invalid("Некорректная история документов")),
        };
        let ledger_path = files::safe_path(root, "work/document-ids.json")?;
        let mut ledger: std::collections::BTreeMap<String, u64> =
            match tokio::fs::read(&ledger_path).await {
                Ok(bytes) => serde_json::from_slice(&bytes).map_err(invalid)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
                Err(e) => return Err(invalid(e)),
            };
        for document in &old {
            let number = document["document_id"]
                .as_str()
                .and_then(|d| d.strip_prefix("document-"))
                .and_then(|d| d.parse::<u64>().ok())
                .ok_or_else(|| invalid("Некорректный ID документа"))?;
            ledger.insert(
                document["asset_id"]
                    .as_str()
                    .ok_or_else(|| invalid("Нет ID материала документа"))?
                    .into(),
                number,
            );
        }
        if ledger.values().any(|n| *n == 0)
            || ledger
                .values()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != ledger.len()
        {
            return Err(invalid("Повреждён журнал ID документов"));
        }
        let mut next = ledger.values().max().copied().unwrap_or(0) + 1;
        let mut documents = Vec::new();
        let mut inputs = Vec::new();
        for asset in assets
            .iter()
            .filter(|a| (a["kind"] == "text" || a["kind"] == "subtitle") && a["path"].is_string())
        {
            let review:Option<String>=sqlx::query_scalar("SELECT r.snapshot_json FROM media_run_sources s JOIN speech_run_reviews l ON l.run_id=s.run_id JOIN speech_text_reviews r ON r.decision_id=l.decision_id WHERE s.asset_id=? ORDER BY s.rowid DESC LIMIT 1").bind(asset["id"].as_str()).fetch_optional(&self.pool).await.map_err(db)?;
            let bytes = if let Some(review) = review {
                serde_json::from_str::<ports::speech_workflow::SpeechReview>(&review)
                    .map_err(invalid)?
                    .source
                    .text
                    .into_bytes()
            } else if asset["kind"] == "text" {
                tokio::fs::read(files::safe_path(
                    root,
                    asset["path"].as_str().unwrap_or_default(),
                )?)
                .await
                .map_err(invalid)?
            } else {
                continue;
            };
            let hash = format!("{:x}", Sha256::digest(&bytes));
            let text = std::str::from_utf8(&bytes).map_err(invalid)?;
            if text.trim().is_empty() {
                continue;
            }
            let number = ledger
                .entry(asset["id"].as_str().unwrap_or_default().into())
                .or_insert_with(|| {
                    let number = next;
                    next += 1;
                    number
                });
            let document = format!("document-{number:06}");
            let input = format!(
                "input/{}-{}.txt",
                asset["id"].as_str().unwrap_or_default(),
                hash
            );
            let destination = files::safe_path(root, &format!("puza/{input}"))?;
            if !destination.exists() {
                files::atomic(&destination, &bytes).await?;
            }
            inputs.push(json!({"document_id":document,"input":input}));
            documents.push(json!({"document_id":document,"asset_id":asset["id"],"sha256":hash}));
        }
        files::atomic(&ledger_path, &serde_json::to_vec(&ledger).map_err(invalid)?).await?;
        Ok((documents, inputs))
    }
}
