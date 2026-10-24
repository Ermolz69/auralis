use super::*;
use crate::{
    local::artifact_store::LocalArtifactStore,
    sqlite::{SqliteArtifactIndex, SqliteProjectRepository, connect_sqlite},
};
use domain::{
    library::*,
    media::{ArtifactId, ArtifactState},
    project::Project,
};
use ports::{
    artifact_index::ArtifactIndex, library::LibraryRepository, repository::ProjectRepository,
    storage::ArtifactStore,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

struct Validator(AtomicBool);
#[async_trait]
impl MediaContractValidator for Validator {
    async fn validate(&self, root: &Path) -> Result<(), PortError> {
        if self.0.load(Ordering::SeqCst) {
            return Err(invalid("CONTRACT unavailable"));
        }
        let value: Value = serde_json::from_slice(
            &tokio::fs::read(root.join("auralis-project.json"))
                .await
                .map_err(invalid)?,
        )
        .map_err(invalid)?;
        for asset in value["assets"]
            .as_array()
            .ok_or_else(|| invalid("No assets"))?
        {
            if let Some(path) = asset["path"].as_str() {
                if files::digest(&root.join(path)).await?.1 != asset["sha256"] {
                    return Err(invalid("Hash mismatch"));
                }
            }
        }
        Ok(())
    }
}
struct Fixture {
    _dir: tempfile::TempDir,
    media: LocalMediaProjects,
    validator: Arc<Validator>,
    source: String,
    projects: Vec<String>,
}
impl Fixture {
    async fn new() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let pool = connect_sqlite(dir.path().join("library.db")).await?;
        let repo = SqliteProjectRepository::new(pool.clone());
        let mut projects = Vec::new();
        for name in ["Library", "First", "Second"] {
            let project = Project::new(name.into())?;
            let snapshot = ports::puza_contract::PuzaContractSnapshot {
                contract_version: "fixture".into(),
                project_json: r#"{"version":1,"documents":[]}"#.into(),
                cast_json: r#"{"version":2,"roles":{}}"#.into(),
                source_text: None,
                bindings_json: "[]".into(),
                sha256: "a".repeat(64),
            };
            projects.push(
                repo.create_with_contract(project, snapshot)
                    .await?
                    .id()
                    .to_string(),
            );
        }
        let source = uuid::Uuid::new_v4().to_string();
        let mut state = LibraryState::default();
        state.standalone_owners.push(projects[0].clone());
        state.assets.push(LibraryAsset {
            id: source.clone(),
            owner_project_id: projects[0].clone(),
            artifact_id: None,
            name: "Original".into(),
            kind: LibraryAssetKind::Text,
            format: Some("txt".into()),
            text: Some("Проверенный текст".into()),
            parent_id: None,
            tags: vec!["tag".into()],
            opened_at: None,
        });
        super::super::library_repository::SqliteLibraryRepository::new(pool.clone())
            .save(state)
            .await?;
        let validator = Arc::new(Validator(AtomicBool::new(false)));
        let media = LocalMediaProjects::new(pool, dir.path().join("projects"), validator.clone());
        Ok(Self {
            _dir: dir,
            media,
            validator,
            source,
            projects,
        })
    }
    async fn finalize(&self, op: &str) -> Result<(), Box<dyn Error>> {
        let (staging,final_key,size,asset):(String,String,i64,String)=sqlx::query_as("SELECT staging_key,final_key,total_bytes,json_extract(asset_json,'$.id') FROM media_copy_operations WHERE id=?").bind(op).fetch_one(&self.media.pool).await?;
        LocalArtifactStore::new(self.media.root.clone())
            .finalize_staged_artifact(&staging, &final_key, Some(size as u64))
            .await?;
        SqliteArtifactIndex::new(self.media.pool.clone())
            .update_state(
                &ArtifactId(uuid::Uuid::parse_str(&asset)?),
                ArtifactState::Ready,
                Some(chrono::Utc::now()),
            )
            .await?;
        Ok(())
    }
    async fn copy(&self, index: usize) -> Result<(String, String), Box<dyn Error>> {
        let op = uuid::Uuid::new_v4().to_string();
        self.media
            .attach(&op, &self.projects[index], &self.source, false)
            .await?;
        self.finalize(&op).await?;
        self.media.reconcile(self.media.state().await?).await?;
        Ok((
            op.clone(),
            self.media
                .operation(&op)
                .await?
                .ok_or("Missing operation")?
                .asset_id,
        ))
    }
}

#[tokio::test]
async fn copies_are_independent_idempotent_and_manifested() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let (_, first) = f.copy(1).await?;
    let (_, second) = f.copy(2).await?;
    assert_ne!(first, second);
    let repeated = uuid::Uuid::new_v4().to_string();
    f.media
        .attach(&repeated, &f.projects[1], &f.source, false)
        .await?;
    assert_eq!(
        f.media
            .operation(&repeated)
            .await?
            .ok_or("Missing operation")?
            .asset_id,
        first
    );
    assert!(
        f.media
            .attach(&repeated, &f.projects[2], &f.source, false)
            .await
            .is_err()
    );
    let a = f.media.structure(&f.projects[1]).await?;
    let b = f.media.structure(&f.projects[2]).await?;
    let pa = f
        .media
        .root
        .join(&f.projects[1])
        .join(a["assets"][0]["path"].as_str().ok_or("No path")?);
    let pb = f
        .media
        .root
        .join(&f.projects[2])
        .join(b["assets"][0]["path"].as_str().ok_or("No path")?);
    tokio::fs::write(&pa, "Changed").await?;
    assert_eq!(tokio::fs::read_to_string(pb).await?, "Проверенный текст");
    assert!(f.media.validate(&f.projects[1]).await.is_err());
    assert!(f.media.validate(&f.projects[2]).await.is_ok());
    Ok(())
}
#[tokio::test]
async fn pending_and_cancelled_copies_are_never_visible() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let op = uuid::Uuid::new_v4().to_string();
    f.media
        .attach(&op, &f.projects[1], &f.source, false)
        .await?;
    let id = f
        .media
        .operation(&op)
        .await?
        .ok_or("No operation")?
        .asset_id;
    assert!(!f.media.state().await?.assets.iter().any(|a| a.id == id));
    f.media.cancel(&op).await?;
    f.finalize(&op).await?;
    let state = f.media.reconcile(f.media.state().await?).await?;
    assert!(!state.assets.iter().any(|a| a.id == id));
    assert!(state.links.is_empty());
    Ok(())
}
#[tokio::test]
async fn contract_failure_keeps_copy_pending_then_recovers() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let op = uuid::Uuid::new_v4().to_string();
    f.media
        .attach(&op, &f.projects[1], &f.source, false)
        .await?;
    f.finalize(&op).await?;
    let copy_id = f
        .media
        .operation(&op)
        .await?
        .ok_or("No operation")?
        .asset_id;
    assert!(
        SqliteArtifactIndex::new(f.media.pool.clone())
            .get(&ArtifactId(uuid::Uuid::parse_str(&copy_id)?))
            .await?
            .is_none()
    );
    f.validator.0.store(true, Ordering::SeqCst);
    f.media.reconcile(f.media.state().await?).await?;
    assert_eq!(
        f.media.operation(&op).await?.ok_or("No operation")?.status,
        "pending"
    );
    assert!(f.media.state().await?.links.is_empty());
    f.validator.0.store(false, Ordering::SeqCst);
    f.media.reconcile(f.media.state().await?).await?;
    assert_eq!(
        f.media.operation(&op).await?.ok_or("No operation")?.status,
        "ready"
    );
    Ok(())
}
#[tokio::test]
async fn linked_project_migrates_without_losing_drafts() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let mut state = f.media.state().await?;
    state.links.push(LibraryLink {
        project_id: f.projects[1].clone(),
        asset_id: f.source.clone(),
        folder_id: None,
    });
    let tab = uuid::Uuid::new_v4().to_string();
    state.workspaces.push(LibraryWorkspace {
        id: format!("project:{}", f.projects[1]),
        project_id: Some(f.projects[1].clone()),
        asset_id: None,
        active_tab: Some(tab.clone()),
        tabs: vec![LibraryTab {
            id: tab,
            tool: LibraryTool::Speech,
            asset_id: Some(f.source.clone()),
            draft: r#"{"text":"draft","selected":"old-run"}"#.into(),
        }],
    });
    state = super::super::library_repository::SqliteLibraryRepository::new(f.media.pool.clone())
        .save(state)
        .await?;
    f.media.reconcile(state).await?;
    let op: String = sqlx::query_scalar("SELECT id FROM media_copy_operations WHERE project_id=?")
        .bind(&f.projects[1])
        .fetch_one(&f.media.pool)
        .await?;
    f.finalize(&op).await?;
    let state = f.media.reconcile(f.media.state().await?).await?;
    assert_ne!(state.links[0].asset_id, f.source);
    assert_eq!(
        state.workspaces[0].tabs[0].asset_id.as_ref(),
        Some(&state.links[0].asset_id)
    );
    assert!(state.workspaces[0].tabs[0].draft.contains("old-run"));
    assert!(
        f.media
            .root
            .join(&f.projects[1])
            .join("work/migration/library-before.json")
            .exists()
    );
    assert_eq!(
        state
            .assets
            .iter()
            .find(|a| a.id == f.source)
            .ok_or("Original lost")?
            .text
            .as_deref(),
        Some("Проверенный текст")
    );
    Ok(())
}
#[tokio::test]
async fn interrupted_copy_and_unsafe_paths_are_rejected() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let op = uuid::Uuid::new_v4().to_string();
    f.media
        .attach(&op, &f.projects[1], &f.source, false)
        .await?;
    sqlx::query("UPDATE media_copy_operations SET state='copying' WHERE id=?")
        .bind(&op)
        .execute(&f.media.pool)
        .await?;
    f.media.reconcile(f.media.state().await?).await?;
    assert_eq!(
        f.media.operation(&op).await?.ok_or("No operation")?.status,
        "failed"
    );
    for path in ["../outside", "C:/outside", "/absolute", "a/../b", "a\\b"] {
        assert!(files::safe_path(&f.media.root, path).is_err());
    }
    Ok(())
}
#[tokio::test]
async fn puza_manifest_hash_matches_configuration_and_document_ids_survive()
-> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let (_, asset) = f.copy(1).await?;
    let before = f.media.structure(&f.projects[1]).await?;
    let root = f.media.root.join(&f.projects[1]);
    let config: Value =
        serde_json::from_slice(&tokio::fs::read(root.join("puza/project.json")).await?)?;
    let manifest: Value =
        serde_json::from_slice(&tokio::fs::read(root.join("puza/manifest.json")).await?)?;
    assert_eq!(
        manifest["project_sha256"],
        json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&config)?)
        ))
    );
    let mut state = f.media.state().await?;
    state
        .assets
        .iter_mut()
        .find(|a| a.id == asset)
        .ok_or("No copy")?
        .name = "Renamed".into();
    state = super::super::library_repository::SqliteLibraryRepository::new(f.media.pool.clone())
        .save(state)
        .await?;
    f.media.reconcile(state).await?;
    let after = f.media.structure(&f.projects[1]).await?;
    assert_eq!(before["documents"], after["documents"]);
    Ok(())
}

#[tokio::test]
async fn damaged_copy_does_not_block_other_projects() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let op = uuid::Uuid::new_v4().to_string();
    f.media
        .attach(&op, &f.projects[1], &f.source, false)
        .await?;
    f.finalize(&op).await?;
    let key: String = sqlx::query_scalar("SELECT final_key FROM media_copy_operations WHERE id=?")
        .bind(&op)
        .fetch_one(&f.media.pool)
        .await?;
    tokio::fs::write(f.media.root.join(key), "damaged").await?;
    f.media.reconcile(f.media.state().await?).await?;
    assert_eq!(
        f.media.operation(&op).await?.ok_or("No operation")?.status,
        "failed"
    );
    let (_, other) = f.copy(2).await?;
    assert!(
        f.media
            .state()
            .await?
            .links
            .iter()
            .any(|l| l.asset_id == other)
    );
    assert!(
        !f.media
            .state()
            .await?
            .links
            .iter()
            .any(|l| l.project_id == f.projects[1])
    );
    Ok(())
}
#[tokio::test]
async fn duplicate_preserves_original_and_repeat_restores_removed_membership()
-> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    let (_, original) = f.copy(1).await?;
    let op = uuid::Uuid::new_v4().to_string();
    f.media.attach(&op, &f.projects[1], &original, true).await?;
    f.finalize(&op).await?;
    f.media.reconcile(f.media.state().await?).await?;
    let duplicate = f
        .media
        .operation(&op)
        .await?
        .ok_or("No operation")?
        .asset_id;
    let mut state = f.media.state().await?;
    assert!(state.links.iter().any(|l| l.asset_id == original));
    assert!(state.links.iter().any(|l| l.asset_id == duplicate));
    state.links.retain(|l| l.asset_id != original);
    super::super::library_repository::SqliteLibraryRepository::new(f.media.pool.clone())
        .save(state)
        .await?;
    let repeated = uuid::Uuid::new_v4().to_string();
    f.media
        .attach(&repeated, &f.projects[1], &f.source, false)
        .await?;
    assert_eq!(
        f.media
            .operation(&repeated)
            .await?
            .ok_or("No repeated")?
            .asset_id,
        original
    );
    assert!(
        f.media
            .state()
            .await?
            .links
            .iter()
            .any(|l| l.asset_id == original)
    );
    Ok(())
}
#[tokio::test]
async fn staging_write_failure_keeps_source_and_old_project_usable() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new().await?;
    f.media.ensure(&f.projects[1]).await?;
    let before = f.media.structure(&f.projects[1]).await?;
    // Deterministic I/O failure at the same boundary used for disk-full errors.
    tokio::fs::write(f.media.root.join(".staging"), "not a directory").await?;
    let op = uuid::Uuid::new_v4().to_string();
    assert!(
        f.media
            .attach(&op, &f.projects[1], &f.source, false)
            .await
            .is_err()
    );
    assert!(f.media.state().await?.links.is_empty());
    assert_eq!(f.media.structure(&f.projects[1]).await?, before);
    assert_eq!(
        f.media.operation(&op).await?.ok_or("No operation")?.status,
        "failed"
    );
    assert!(
        f.media
            .state()
            .await?
            .assets
            .iter()
            .any(|a| a.id == f.source)
    );
    Ok(())
}
