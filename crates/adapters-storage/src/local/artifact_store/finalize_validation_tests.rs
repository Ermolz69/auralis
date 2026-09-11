#![allow(clippy::unwrap_used)]

use super::LocalArtifactStore;
use domain::{media::ArtifactKind, project::ProjectId};
use ports::storage::ArtifactStore;

#[tokio::test]
async fn final_path_must_be_a_regular_file_of_the_expected_size() {
    for directory in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("external.mp4");
        std::fs::write(&source, b"original content").unwrap();
        let base = root.path().join("artifacts");
        std::fs::create_dir(&base).unwrap();
        let store = LocalArtifactStore::new(base.clone());
        let staged = store
            .import_external_file(&ProjectId::new(), ArtifactKind::SourceVideo, &source, None)
            .await
            .unwrap();
        let final_path = base.join(&staged.final_key);
        std::fs::create_dir_all(final_path.parent().unwrap()).unwrap();
        if directory {
            std::fs::create_dir(&final_path).unwrap();
        } else {
            std::fs::write(&final_path, b"wrong").unwrap();
        }
        assert!(
            store
                .finalize_staged_artifact(
                    &staged.staging_key,
                    &staged.final_key,
                    Some(staged.size_bytes)
                )
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&source).unwrap(), b"original content");
        assert_eq!(
            std::fs::read(base.join(&staged.staging_key)).unwrap(),
            b"original content"
        );
        if directory {
            assert!(final_path.is_dir());
        } else {
            assert_eq!(std::fs::read(&final_path).unwrap(), b"wrong");
        }
    }
}

#[tokio::test]
async fn truncated_staging_is_not_moved_or_reported_as_finalized() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("external.mp4");
    std::fs::write(&source, b"original content").unwrap();
    let store = LocalArtifactStore::new(root.path().to_path_buf());
    let staged = store
        .import_external_file(&ProjectId::new(), ArtifactKind::SourceVideo, &source, None)
        .await
        .unwrap();
    std::fs::write(root.path().join(&staged.staging_key), b"partial").unwrap();
    assert!(
        store
            .finalize_staged_artifact(
                &staged.staging_key,
                &staged.final_key,
                Some(staged.size_bytes)
            )
            .await
            .is_err()
    );
    assert!(!root.path().join(staged.final_key).exists());
    assert!(root.path().join(staged.staging_key).is_file());
}
