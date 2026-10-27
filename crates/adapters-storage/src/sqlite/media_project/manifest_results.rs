use super::{LocalMediaProjects, db, files, invalid};
use ports::error::PortError;
use serde_json::{Value, json};
use sqlx::Row;

impl LocalMediaProjects {
    pub(super) async fn manifest_results(
        &self,
        id: &str,
        assets: &[Value],
    ) -> Result<Vec<Value>, PortError> {
        let mut outputs: Vec<(String, String, String, String, Option<String>)> = Vec::new();
        let speech=sqlx::query("SELECT p.run_id,p.publication_json,s.asset_id,r.snapshot_json FROM speech_publications p JOIN speech_run_intents i ON i.run_id=p.run_id LEFT JOIN media_run_sources s ON s.run_id=p.run_id LEFT JOIN speech_run_reviews l ON l.run_id=p.run_id LEFT JOIN speech_text_reviews r ON r.decision_id=l.decision_id WHERE i.project_id=? AND p.ready=1")
            .bind(id).fetch_all(&self.pool).await.map_err(db)?;
        for row in speech {
            let publication: ports::speech_workflow::SpeechPublication =
                serde_json::from_str(&row.try_get::<String, _>("publication_json").map_err(db)?)
                    .map_err(invalid)?;
            let bound: Option<String> = row.try_get("asset_id").map_err(db)?;
            let snapshot: Option<String> = row.try_get("snapshot_json").map_err(db)?;
            let review: Value = snapshot
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(Value::Null);
            let source = &review["source"];
            let selected = assets
                .iter()
                .find(|a| bound.as_ref().is_some_and(|b| a["id"] == *b))
                .or_else(|| {
                    assets.iter().find(|a| {
                        (!source["source_id"].is_null() && a["artifact_id"] == source["source_id"])
                            || (!source["sha256"].is_null() && a["sha256"] == source["sha256"])
                    })
                });
            let fallback = assets.iter().find(|a| {
                publication
                    .files
                    .iter()
                    .any(|f| a["artifact_id"] == f.artifact_id.to_string())
            });
            let Some(asset) = selected.or(fallback) else {
                continue;
            };
            for file in publication.files {
                outputs.push((
                    asset["id"].as_str().unwrap_or_default().into(),
                    publication.run_id.to_string(),
                    file.artifact_id.to_string(),
                    file.kind,
                    Some(file.sha256),
                ));
            }
        }
        let translations=sqlx::query("SELECT p.run_id,p.artifact_id,t.source_artifact_id FROM translation_publications p JOIN project_translations t ON t.translation_id=p.translation_id WHERE t.project_id=? AND p.state='ready'")
            .bind(id).fetch_all(&self.pool).await.map_err(db)?;
        for row in translations {
            let source: String = row.try_get("source_artifact_id").map_err(db)?;
            let output: String = row.try_get("artifact_id").map_err(db)?;
            if let Some(asset) = assets
                .iter()
                .find(|a| a["artifact_id"] == source)
                .or_else(|| assets.iter().find(|a| a["artifact_id"] == output))
            {
                outputs.push((
                    asset["id"].as_str().unwrap_or_default().into(),
                    row.try_get("run_id").map_err(db)?,
                    output,
                    "translatedSubtitle".into(),
                    None,
                ));
            }
        }
        let root = files::safe_path(&self.root, id)?;
        let mut results = Vec::new();
        for (asset, run, artifact, kind, expected) in outputs {
            let key:Option<String>=sqlx::query_scalar("SELECT location_value FROM artifacts WHERE id=? AND project_id=? AND state='ready'")
                .bind(&artifact).bind(id).fetch_optional(&self.pool).await.map_err(db)?;
            let source = files::safe_path(
                &self.root,
                &key.ok_or_else(|| invalid("Результат ещё не опубликован"))?,
            )?;
            let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("bin");
            let path = format!("results/{asset}/{run}/{artifact}.{ext}");
            let (size, hash) =
                files::copy_verified(&source, &files::safe_path(&root, &path)?).await?;
            if expected.is_some_and(|e| e != hash) {
                return Err(invalid("Хеш сохранённого результата изменён"));
            }
            results.push(json!({"id":artifact,"asset_id":asset,"run_id":run,"kind":kind,"path":path,"sha256":hash,"bytes":size}));
        }
        Ok(results)
    }
}
