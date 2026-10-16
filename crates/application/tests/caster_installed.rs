#![allow(clippy::unwrap_used, clippy::expect_used)]

use adapters_caster::{CasterRuntimeConfig, CasterWorkerAdapter};
use adapters_storage::memory::{
    InMemoryArtifactIndex, InMemoryDatabase, InMemoryProjectRepository,
};
use application::usecases::caster::CasterUseCases;
use application::usecases::project::lifecycle::ProjectLifecycleLocks;
use domain::media::{ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::caster::{CastRole, LogicalCast, ManualSelection};
use ports::repository::ProjectRepository;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[tokio::test]
#[ignore = "requires an explicitly installed Caster runtime and isolated qualified fixture"]
async fn installed_worker_publishes_a_managed_cast_and_survives_restart() {
    let python =
        PathBuf::from(std::env::var_os("AURALIS_TEST_CASTER_PYTHON").expect("installed Python"));
    let voice_store = PathBuf::from(
        std::env::var_os("AURALIS_TEST_CASTER_VOICE_STORE").expect("isolated voice store"),
    );
    assert!(python.is_file());
    assert!(voice_store.is_dir());

    let root = tempfile::tempdir().unwrap();
    let database = Arc::new(Mutex::new(InMemoryDatabase::default()));
    let projects = Arc::new(InMemoryProjectRepository::new(database));
    let project = projects
        .create(Project::new("Installed Caster".into()).unwrap())
        .await
        .unwrap();
    let project_id = project.id();
    let artifacts = Arc::new(InMemoryArtifactIndex::new());
    let runtime = CasterRuntimeConfig {
        python_executable: python,
        voice_store,
    };
    let make_usecases = || {
        CasterUseCases::new(
            projects.clone(),
            artifacts.clone(),
            Arc::new(CasterWorkerAdapter::new(
                Some(runtime.clone()),
                root.path().join("projects"),
            )),
            Arc::new(ProjectLifecycleLocks::new()),
        )
    };
    let caster = make_usecases();

    let page = caster.catalog_page(project_id, None, 20).await.unwrap();
    let entry = page
        .items
        .iter()
        .find(|item| {
            item.state == "valid"
                && item
                    .qualified_capabilities
                    .as_ref()
                    .is_some_and(|items| !items.is_empty())
        })
        .expect("qualified fixture voice");
    let details = caster
        .inspect_voice(project_id, entry.voice_id.clone(), None, 20)
        .await
        .unwrap();
    let reference = details
        .references
        .iter()
        .find(|item| item.qualification_status == "qualified")
        .expect("qualified fixture reference");
    let compatible = caster
        .check_compatibility(
            project_id,
            entry.voice_id.clone(),
            reference.engine_id.clone(),
            reference.emotion_id.clone(),
        )
        .await
        .unwrap();
    assert!(compatible.compatible);
    assert!(compatible.reference_ids.contains(&reference.reference_id));
    let inspected = caster
        .inspect_reference(
            project_id,
            entry.voice_id.clone(),
            reference.reference_id.clone(),
        )
        .await
        .unwrap();
    let source_audio = std::fs::read(&inspected.audio_path).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&source_audio)),
        inspected.audio_sha256
    );

    let cast = LogicalCast {
        version: 2,
        roles: BTreeMap::from([(
            "narrator".into(),
            CastRole {
                display_name: "Narrator".into(),
                voice_id: Some(entry.voice_id.clone()),
            },
        )]),
    };
    let selections = BTreeMap::from([(
        "narrator".into(),
        ManualSelection {
            voice_id: entry.voice_id.clone(),
            reference_id: reference.reference_id.clone(),
            engine_id: reference.engine_id.clone(),
            emotion_id: reference.emotion_id.clone(),
        },
    )]);
    let saved = caster
        .assign_roles(
            project_id,
            cast,
            selections,
            None,
            "installed-fixture-request".into(),
        )
        .await
        .unwrap();
    let indexed = artifacts
        .list_by_project_and_kind(project_id, ArtifactKind::ManualCast)
        .await
        .unwrap();
    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].state, ArtifactState::Ready);
    let ArtifactLocation::StorageKey(key) = &indexed[0].location else {
        panic!("cast must be managed");
    };
    let revision = root.path().join("projects").join(key);
    assert!(revision.is_file());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&revision).unwrap())),
        saved.revision_sha256
    );
    assert_eq!(std::fs::read(&inspected.audio_path).unwrap(), source_audio);

    drop(caster);
    let restarted = make_usecases();
    let current = restarted.current_cast(project_id).await.unwrap().unwrap();
    assert_eq!(current.revision_sha256, saved.revision_sha256);
    assert_eq!(
        artifacts
            .list_by_project_and_kind(project_id, ArtifactKind::ManualCast)
            .await
            .unwrap()
            .len(),
        1
    );

    let replaced = projects
        .create(Project::new("Replaced cast path".into()).unwrap())
        .await
        .unwrap();
    std::fs::write(
        root.path().join("projects").join(replaced.id().to_string()),
        b"not a directory",
    )
    .unwrap();
    assert!(restarted.current_cast(replaced.id()).await.is_err());
    assert!(restarted.recover_cast(replaced.id()).await.is_err());
}
