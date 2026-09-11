#![allow(clippy::unwrap_used)]

use super::LocalArtifactStore;
use domain::{media::ArtifactKind, project::ProjectId};
use ports::storage::ArtifactStore;

#[tokio::test]
async fn readable_readonly_external_original_is_preserved() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("readonly.mp4");
    std::fs::write(&source, b"read-only original").unwrap();
    let original_permissions = std::fs::metadata(&source).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        readonly.set_mode(0o444);
    }
    #[cfg(not(unix))]
    readonly.set_readonly(true);
    std::fs::set_permissions(&source, readonly.clone()).unwrap();
    let readonly = std::fs::metadata(&source).unwrap().permissions();
    let base = root.path().join("artifacts");
    let store = LocalArtifactStore::new(base.clone());
    let staged = store
        .import_external_file(&ProjectId::new(), ArtifactKind::SourceVideo, &source, None)
        .await
        .unwrap();
    store
        .finalize_staged_artifact(
            &staged.staging_key,
            &staged.final_key,
            Some(staged.size_bytes),
        )
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(base.join(staged.final_key)).unwrap(),
        b"read-only original"
    );
    assert_eq!(std::fs::read(&source).unwrap(), b"read-only original");
    assert_eq!(std::fs::metadata(&source).unwrap().permissions(), readonly);
    std::fs::set_permissions(&source, original_permissions).unwrap();
}
