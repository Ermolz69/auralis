#![allow(clippy::unwrap_used)]

use super::LocalArtifactStore;
use domain::media::{ArtifactKind, ArtifactLocation};
use domain::project::ProjectId;
use ports::storage::ArtifactStore;

async fn assert_extensions(owned: bool) {
    let cases = [
        ("download.mp4", Some("original"), "mp4"),
        ("download.webm", Some("original"), "webm"),
        ("download.mkv", Some("original"), "mkv"),
        ("download.webm", Some("original.mp4"), "mp4"),
        ("download.mp4", None, "mp4"),
        ("download.mp4", Some(""), "mp4"),
        ("download", Some("original"), "bin"),
        ("download", None, "bin"),
    ];

    for (filename, hint, expected) in cases {
        let root = tempfile::tempdir().unwrap();
        let source_dir = tempfile::tempdir().unwrap();
        let store = LocalArtifactStore::new(root.path().to_path_buf());
        let source_path = source_dir.path().join(filename);
        tokio::fs::write(&source_path, b"video data").await.unwrap();

        let staged = if owned {
            store
                .stage_owned_temp_file(
                    &ProjectId::new(),
                    ArtifactKind::DownloadedVideo,
                    &source_path,
                    hint,
                )
                .await
        } else {
            store
                .import_external_file(
                    &ProjectId::new(),
                    ArtifactKind::SourceVideo,
                    &source_path,
                    hint,
                )
                .await
        }
        .unwrap();

        let suffix = format!(".{expected}");
        assert!(
            staged.staging_key.ends_with(&suffix),
            "{filename}, {hint:?}"
        );
        assert!(staged.final_key.ends_with(&suffix), "{filename}, {hint:?}");
        assert_eq!(
            staged.artifact.location,
            ArtifactLocation::StorageKey(staged.final_key.clone())
        );
        assert_eq!(source_path.exists(), !owned);
        store
            .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read(store.resolve_storage_key(&staged.final_key).unwrap())
                .await
                .unwrap(),
            b"video data"
        );
        if !owned {
            assert_eq!(tokio::fs::read(&source_path).await.unwrap(), b"video data");
        }
    }
}

#[tokio::test]
async fn owned_staging_preserves_source_extension_without_hint_extension() {
    assert_extensions(true).await;
}

#[tokio::test]
async fn external_import_preserves_source_extension_without_hint_extension() {
    assert_extensions(false).await;
}
