use adapters_storage::sqlite::{
    SqliteArtifactIndex, SqliteProjectRepository, connect_sqlite,
    library_repository::SqliteLibraryRepository,
};
use application::usecases::library::LibraryUseCase;
use domain::{
    library::{
        LibraryAsset, LibraryAssetKind, LibraryFolder, LibraryLink, LibraryState, LibraryTab,
        LibraryTool, LibraryWorkspace,
    },
    project::Project,
};
use ports::{library::LibraryRepository, repository::ProjectRepository};
use std::error::Error;
use std::sync::Arc;

fn text(owner: &Project) -> LibraryAsset {
    LibraryAsset {
        id: uuid::Uuid::new_v4().to_string(),
        owner_project_id: owner.id().to_string(),
        name: "Standalone text".into(),
        kind: LibraryAssetKind::Text,
        text: Some("Проверенный исходник".into()),
        artifact_id: None,
        format: Some("txt".into()),
        parent_id: None,
        tags: vec!["test".into()],
        opened_at: None,
    }
}

#[tokio::test]
async fn standalone_assets_shared_membership_tabs_and_drafts_survive_reopen()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("library.sqlite");
    let pool = connect_sqlite(&path).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let owner = projects.create(Project::new("Owner".into())?).await?;
    let project = projects.create(Project::new("Project".into())?).await?;
    let repository = Arc::new(SqliteLibraryRepository::new(pool.clone()));
    let service = LibraryUseCase::new(
        repository.clone(),
        projects,
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
    );
    let asset = text(&owner);
    let mut state = service.load().await?;
    state.standalone_owners.push(owner.id().to_string());
    state.assets.push(asset.clone());
    let folder = LibraryFolder {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id().to_string(),
        parent_id: None,
        name: "Chapter".into(),
    };
    state.folders.push(folder.clone());
    state.links.push(LibraryLink {
        asset_id: asset.id.clone(),
        project_id: project.id().to_string(),
        folder_id: Some(folder.id),
    });
    let tab = LibraryTab {
        id: uuid::Uuid::new_v4().to_string(),
        tool: LibraryTool::Speech,
        asset_id: Some(asset.id.clone()),
        draft: "{\"text\":\"Черновик\"}".into(),
    };
    state.workspaces.push(LibraryWorkspace {
        id: format!("asset:{}", asset.id),
        asset_id: Some(asset.id.clone()),
        project_id: None,
        active_tab: Some(tab.id.clone()),
        tabs: vec![tab],
    });
    let saved = service.save(state.clone()).await?;
    assert_eq!(saved.revision, 1);
    assert!(service.save(state).await.is_err(), "stale writes must fail");
    let reopened = connect_sqlite(&path).await?;
    let restored = SqliteLibraryRepository::new(reopened).load().await?;
    assert_eq!(restored.links.len(), 1);
    assert_eq!(
        restored.workspaces[0].tabs[0].draft,
        "{\"text\":\"Черновик\"}"
    );
    let mut unlinked = restored;
    unlinked.links.clear();
    service.save(unlinked).await?;
    assert_eq!(
        service.load().await?.assets.len(),
        1,
        "removing membership preserves the asset"
    );
    Ok(())
}

#[tokio::test]
async fn missing_owners_and_unready_artifacts_cannot_be_registered() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("library.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let owner = projects.create(Project::new("Owner".into())?).await?;
    let service = LibraryUseCase::new(
        Arc::new(SqliteLibraryRepository::new(pool.clone())),
        projects,
        Arc::new(SqliteArtifactIndex::new(pool)),
    );
    let mut state = LibraryState::default();
    let mut asset = text(&owner);
    asset.artifact_id = Some(uuid::Uuid::new_v4().to_string());
    state.assets.push(asset);
    assert!(service.save(state.clone()).await.is_err());
    state.assets[0].artifact_id = None;
    state.assets[0].owner_project_id = uuid::Uuid::new_v4().to_string();
    assert!(service.save(state).await.is_err());
    Ok(())
}

#[test]
fn folder_cycles_and_dangling_tabs_are_rejected() {
    let id = uuid::Uuid::new_v4().to_string();
    let mut state = LibraryState::default();
    state.folders.push(LibraryFolder {
        id: id.clone(),
        project_id: uuid::Uuid::new_v4().to_string(),
        parent_id: Some(id),
        name: "Cycle".into(),
    });
    assert!(state.validate().is_err());
    state.folders.clear();
    state.links.push(LibraryLink {
        asset_id: uuid::Uuid::new_v4().to_string(),
        project_id: uuid::Uuid::new_v4().to_string(),
        folder_id: None,
    });
    assert!(state.validate().is_err());
}

#[tokio::test]
async fn version_seventeen_upgrade_preserves_projects_and_rejects_corrupt_catalog()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("upgrade.sqlite");
    let pool = connect_sqlite(&path).await?;
    let projects = SqliteProjectRepository::new(pool.clone());
    let project = projects
        .create(Project::new("Existing project".into())?)
        .await?;
    sqlx::raw_sql("DROP TABLE media_run_sources; DROP TABLE media_copy_operations; DROP TABLE media_project_receipts; DROP TABLE workspace_library; DROP TABLE audio_assembly_artifacts; DROP TABLE audio_assembly_publications; DROP TABLE audio_assembly_discarded_artifacts; DROP TABLE audio_assembly_attempts; DROP TABLE audio_assembly_intents")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA user_version=17").execute(&pool).await?;
    pool.close().await;
    let pool = connect_sqlite(&path).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA user_version")
            .fetch_one(&pool)
            .await?,
        22
    );
    assert!(
        SqliteProjectRepository::new(pool.clone())
            .get(project.id())
            .await?
            .is_some()
    );
    let repository = SqliteLibraryRepository::new(pool.clone());
    repository.save(LibraryState::default()).await?;
    sqlx::query("UPDATE workspace_library SET payload='{broken' WHERE id=1")
        .execute(&pool)
        .await?;
    assert!(
        repository.load().await.is_err(),
        "corruption must not become an empty catalog"
    );
    assert!(
        SqliteProjectRepository::new(pool.clone())
            .get(project.id())
            .await?
            .is_some()
    );
    Ok(())
}

#[tokio::test]
async fn library_hydrates_only_ready_results_and_retains_unlinked_results()
-> Result<(), Box<dyn Error>> {
    use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
    use ports::artifact_index::ArtifactIndex;
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("results.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let project = projects.create(Project::new("Speech".into())?).await?;
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let now = domain::chrono::Utc::now();
    let wav = Artifact {
        id: ArtifactId::new(),
        kind: ArtifactKind::GeneratedSpeech,
        location: ArtifactLocation::StorageKey("speech/output.wav".into()),
        size_bytes: Some(100),
        state: ArtifactState::PendingFinalize,
        created_at: now,
        updated_at: now,
        ready_at: None,
    };
    artifacts.add(project.id(), &wav).await?;
    let service = LibraryUseCase::new(
        Arc::new(SqliteLibraryRepository::new(pool)),
        projects,
        artifacts.clone(),
    );
    assert!(service.load().await?.assets.is_empty());
    artifacts
        .update_state(&wav.id, ArtifactState::Ready, Some(now))
        .await?;
    let mut state = service.load().await?;
    assert_eq!(state.assets.len(), 1);
    assert_eq!(state.assets[0].kind, LibraryAssetKind::Audio);
    assert_eq!(state.links.len(), 1);
    state.links.clear();
    service.save(state).await?;
    assert!(
        service.load().await?.links.is_empty(),
        "refresh must not reattach an explicitly unlinked asset"
    );
    Ok(())
}

#[tokio::test]
async fn downloaded_source_upgrades_its_placeholder_without_duplicating_the_asset()
-> Result<(), Box<dyn Error>> {
    use domain::media::{
        Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState, MediaSource,
    };
    use ports::artifact_index::ArtifactIndex;
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("download.sqlite")).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let mut project = Project::new("Download".into())?;
    project.import_source(
        MediaSource::YoutubeUrl {
            url: "https://youtube.com/watch?v=fixture".into(),
        },
        None,
    )?;
    let project = projects.create(project).await?;
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let service = LibraryUseCase::new(
        Arc::new(SqliteLibraryRepository::new(pool)),
        projects,
        artifacts.clone(),
    );
    let state = service.load().await?;
    let placeholder = state.assets[0].id.clone();
    service.save(state).await?;
    let now = domain::chrono::Utc::now();
    let video = Artifact {
        id: ArtifactId::new(),
        kind: ArtifactKind::DownloadedVideo,
        location: ArtifactLocation::StorageKey("source/video.mp4".into()),
        size_bytes: Some(100),
        state: ArtifactState::Ready,
        created_at: now,
        updated_at: now,
        ready_at: Some(now),
    };
    artifacts.add(project.id(), &video).await?;
    let upgraded = service.load().await?;
    assert_eq!(upgraded.assets.len(), 1);
    assert_eq!(upgraded.assets[0].id, placeholder);
    assert_eq!(upgraded.assets[0].artifact_id, Some(video.id.to_string()));
    service.save(upgraded).await?;
    assert_eq!(service.load().await?.assets.len(), 1);
    Ok(())
}
