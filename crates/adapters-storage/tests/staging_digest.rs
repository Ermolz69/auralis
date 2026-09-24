use adapters_storage::local::LocalArtifactStore;
use domain::media::ArtifactKind;
use domain::project::ProjectId;
use ports::storage::ArtifactStore;
use sha2::{Digest, Sha256};
use std::error::Error;

#[tokio::test]
async fn staging_digest_detects_same_size_corruption() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("translated.srt");
    let bytes = b"1\n00:00:01,000 --> 00:00:02,000\nRussian line\n";
    std::fs::write(&source, bytes)?;
    let store = LocalArtifactStore::new(directory.path().join("artifacts"));
    let staged = store
        .import_external_file(
            &ProjectId::new(),
            ArtifactKind::TranslatedTranscript,
            &source,
            Some("translated.srt"),
        )
        .await?;
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    assert!(
        store
            .verify_staging_digest(&staged.staging_key, staged.size_bytes, &sha256)
            .await?
    );
    let staged_path = store.resolve_storage_key(&staged.staging_key)?;
    let mut changed = std::fs::read(&staged_path)?;
    changed[0] ^= 1;
    std::fs::write(&staged_path, changed)?;
    assert!(
        !store
            .verify_staging_digest(&staged.staging_key, staged.size_bytes, &sha256)
            .await?
    );
    assert_eq!(std::fs::read(source)?, bytes);
    Ok(())
}
