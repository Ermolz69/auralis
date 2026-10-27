use super::{LocalMediaProjects, db, files, invalid};
use domain::library::LibraryState;
use ports::error::PortError;
use serde_json::{Value, json};

impl LocalMediaProjects {
    pub(super) async fn manifest_tracks(
        &self,
        id: &str,
        state: &LibraryState,
        assets: &[Value],
        previous: &Value,
    ) -> Result<Vec<Value>, PortError> {
        let mut tracks = Vec::new();
        for asset in assets.iter().filter(|a| a["kind"] != "text") {
            let original_id = asset["source_asset_id"]
                .as_str()
                .or(asset["id"].as_str())
                .ok_or_else(|| invalid("Нет ID исходного материала дорожки"))?;
            let original = state.assets.iter().find(|a| a.id == original_id);
            let owner = original.map(|a| a.owner_project_id.as_str()).unwrap_or(id);
            let (metadata, language): (Option<String>, Option<String>) =
                sqlx::query_as("SELECT metadata_json,source_language FROM projects WHERE id=?")
                    .bind(owner)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(db)?;
            let is_source: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id=? AND kind IN ('SourceVideo','DownloadedVideo'))")
                .bind(original.and_then(|a| a.artifact_id.as_deref())).fetch_one(&self.pool).await.map_err(db)?;
            let metadata = if is_source { metadata } else { None };
            let metadata: Value = metadata
                .map(|m| serde_json::from_str(&m))
                .transpose()
                .map_err(invalid)?
                .unwrap_or(Value::Null);
            let mut track = previous["tracks"].as_array().and_then(|items| items.iter().find(|t| t["asset_id"] == asset["id"])).cloned()
                .unwrap_or_else(|| json!({"id":asset["id"],"asset_id":asset["id"],"media_asset_id":null,"kind":asset["kind"],"language":null,"offset_ms":0,"start_ms":null,"end_ms":null}));
            if track["language"].is_null() {
                track["language"] = json!(language);
            }
            if track["start_ms"].is_null() {
                let window = if asset["kind"] == "subtitle" {
                    if let Some(path) = asset["path"].as_str() {
                        let bytes = tokio::fs::read(files::safe_path(&self.root.join(id), path)?)
                            .await
                            .map_err(invalid)?;
                        subtitle_window(&String::from_utf8_lossy(&bytes))
                    } else {
                        None
                    }
                } else {
                    metadata["duration_ms"]
                        .as_u64()
                        .map(|duration| (0, duration))
                };
                if let Some((start, end)) = window {
                    track["start_ms"] = json!(start);
                    track["end_ms"] = json!(end);
                }
            }
            if asset["kind"] == "subtitle" && track["media_asset_id"].is_null() {
                let related: Vec<_> = assets
                    .iter()
                    .filter(|candidate| {
                        candidate["kind"] == "video" || candidate["kind"] == "audio"
                    })
                    .filter(|candidate| {
                        let source = candidate["source_asset_id"]
                            .as_str()
                            .or(candidate["id"].as_str());
                        state
                            .assets
                            .iter()
                            .any(|a| Some(a.id.as_str()) == source && a.owner_project_id == owner)
                    })
                    .collect();
                if related.len() == 1 {
                    track["media_asset_id"] = related[0]["id"].clone();
                }
            }
            if !track["media_asset_id"].is_null()
                && !assets.iter().any(|a| a["id"] == track["media_asset_id"])
            {
                track["media_asset_id"] = Value::Null;
            }
            tracks.push(track);
        }
        Ok(tracks)
    }
}

fn subtitle_window(text: &str) -> Option<(u64, u64)> {
    let windows: Vec<_> = text
        .lines()
        .filter_map(|line| {
            let (start, end) = line.split_once("-->")?;
            Some((
                timestamp(start.trim())?,
                timestamp(end.split_whitespace().next()?)?,
            ))
        })
        .filter(|(start, end)| end >= start)
        .collect();
    Some((
        windows.iter().map(|w| w.0).min()?,
        windows.iter().map(|w| w.1).max()?,
    ))
}
fn timestamp(text: &str) -> Option<u64> {
    let normalized = text.replace(',', ".");
    let parts: Vec<_> = normalized.split(':').collect();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }
    let (seconds, fraction) = parts.last()?.split_once('.')?;
    if fraction.len() != 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let seconds = seconds.parse::<u64>().ok()?;
    let minutes = parts[parts.len() - 2].parse::<u64>().ok()?;
    if minutes >= 60 || seconds >= 60 {
        return None;
    }
    let hours = if parts.len() == 3 {
        parts[0].parse::<u64>().ok()?
    } else {
        0
    };
    hours
        .checked_mul(3_600_000)?
        .checked_add(minutes * 60_000 + seconds * 1_000 + fraction.parse::<u64>().ok()?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timing_windows_preserve_srt_and_webvtt_boundaries() {
        assert_eq!(
            subtitle_window(
                "1\n00:00:01,250 --> 00:00:03,000\nТекст\n\n2\n00:00:04,000 --> 00:00:06,100"
            ),
            Some((1250, 6100))
        );
        assert_eq!(
            subtitle_window("WEBVTT\n\n00:01.250 --> 00:03.000 align:start\nText"),
            Some((1250, 3000))
        );
        assert_eq!(subtitle_window("bad --> 00:00.000"), None);
    }
}
