use adapters_storage::local::LocalArtifactStore;
use adapters_storage::memory::{
    InMemoryArtifactIndex, InMemoryDatabase, InMemoryProjectRepository,
};
use adapters_translate::FormatInspector;
use application::usecases::translation::{
    InspectTranslationSourceRequest, InspectTranslationSourceUseCase,
};
use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::translation_inspection::TranslationSourceFormat;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn project_inspection_extracts_only_ready_owned_original() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
    let projects = Arc::new(InMemoryProjectRepository::new(Arc::new(Mutex::new(
        InMemoryDatabase::new(),
    ))));
    let artifacts = Arc::new(InMemoryArtifactIndex::new());
    let project = projects
        .create(Project::new("Translation source".into())?)
        .await?;
    let source = b"WEBVTT\r\n\r\nNOTE provenance\r\nsynthetic\r\n\r\n00:01.000 --> 00:02.000\r\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\r\n";
    let key = format!("projects/{}/original.vtt", project.id());
    let source_path = store.resolve_storage_key(&key)?;
    std::fs::create_dir_all(source_path.parent().ok_or("source has no parent")?)?;
    std::fs::write(&source_path, source)?;
    let now = domain::chrono::Utc::now();
    let original = Artifact {
        id: ArtifactId::new(),
        kind: ArtifactKind::OriginalSubtitle,
        location: ArtifactLocation::StorageKey(key),
        size_bytes: Some(u64::try_from(source.len())?),
        state: ArtifactState::Ready,
        created_at: now,
        updated_at: now,
        ready_at: Some(now),
    };
    artifacts.add(project.id(), &original).await?;
    let foreign_project = projects
        .create(Project::new("Another project".into())?)
        .await?;
    let use_case = InspectTranslationSourceUseCase::new(
        projects,
        artifacts.clone(),
        store,
        Arc::new(FormatInspector),
    );
    let request = || InspectTranslationSourceRequest {
        project_id: project.id().clone(),
        source_artifact_id: original.id.clone(),
        format: TranslationSourceFormat::Vtt,
    };
    let inspection = use_case.execute(request()).await?;
    assert_eq!(inspection.segments.len(), 1);
    assert_eq!(inspection.segments[0].source_lines, ["你好。"]);
    assert_eq!(inspection.segments[0].cue_label, None);
    assert_eq!(std::fs::read(&source_path)?, source);

    let mut foreign = request();
    foreign.project_id = foreign_project.id().clone();
    assert!(use_case.execute(foreign).await.is_err());
    artifacts
        .update_state(&original.id, ArtifactState::PendingFinalize, None)
        .await?;
    assert!(use_case.execute(request()).await.is_err());
    assert_eq!(std::fs::read(&source_path)?, source);
    artifacts
        .update_state(&original.id, ArtifactState::Ready, Some(now))
        .await?;
    std::fs::write(&source_path, [source.as_slice(), b"\r\n"].concat())?;
    assert!(use_case.execute(request()).await.is_err());
    Ok(())
}
