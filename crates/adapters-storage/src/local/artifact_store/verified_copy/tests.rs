use super::*;
use crate::local::artifact_store::LocalArtifactStore;
use domain::{media::ArtifactKind, project::ProjectId, speech::Sha256Digest};
use ports::storage::ArtifactStore;
use std::{
    io::Cursor,
    pin::Pin,
    task::{Context, Poll},
};

fn request(source: PathBuf, bytes: &[u8]) -> Result<VerifiedArtifactCopy, &'static str> {
    Ok(VerifiedArtifactCopy {
        project_id: ProjectId::new(),
        kind: ArtifactKind::GeneratedSpeech,
        source_path: source,
        filename_hint: Some("assembled.wav".into()),
        expected_sha256: Sha256Digest::parse(format!("{:x}", Sha256::digest(bytes)))?,
        max_bytes: 1024 * 1024,
    })
}

#[tokio::test]
async fn verified_copy_keeps_original_and_returns_only_pending_managed_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("raw.wav");
    let bytes = vec![7_u8; 150000];
    tokio::fs::write(&source, &bytes).await?;
    let permissions = tokio::fs::metadata(&source).await?.permissions();
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    tokio::fs::set_permissions(&source, readonly).await?;
    let store = LocalArtifactStore::new(root.path().join("managed"));
    let staged = store
        .stage_verified_copy(request(source.clone(), &bytes)?, CancellationToken::new())
        .await?;
    assert_eq!(staged.artifact.state, ArtifactState::PendingFinalize);
    assert_eq!(staged.size_bytes, 150000);
    assert_eq!(
        tokio::fs::read(store.resolve_storage_key(&staged.staging_key)?).await?,
        bytes
    );
    assert!(!store.resolve_storage_key(&staged.final_key)?.exists());
    assert_eq!(tokio::fs::read(&source).await?, bytes);
    tokio::fs::set_permissions(&source, permissions).await?;
    Ok(())
}

#[tokio::test]
async fn changed_hash_cleans_only_its_unaccepted_staged_copy()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("raw.wav");
    tokio::fs::write(&source, b"original").await?;
    let mut copy = request(source.clone(), b"changed")?;
    copy.max_bytes = 100;
    let store = LocalArtifactStore::new(root.path().join("managed"));
    assert!(
        store
            .stage_verified_copy(copy, CancellationToken::new())
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(source).await?, b"original");
    assert_eq!(
        std::fs::read_dir(root.path().join("managed/.staging"))?.count(),
        0
    );
    Ok(())
}

#[tokio::test]
async fn cancellation_and_byte_budget_refusal_do_not_allocate_a_staging_file()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("raw.wav");
    tokio::fs::write(&source, b"original").await?;
    let store = LocalArtifactStore::new(root.path().join("managed"));
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        store
            .stage_verified_copy(request(source.clone(), b"original")?, cancel)
            .await,
        Err(PortError::Cancelled)
    ));
    let mut copy = request(source, b"original")?;
    copy.max_bytes = 7;
    assert!(
        store
            .stage_verified_copy(copy, CancellationToken::new())
            .await
            .is_err()
    );
    assert!(!root.path().join("managed/.staging").exists());
    Ok(())
}

struct CancellingWriter {
    bytes: Vec<u8>,
    cancel: CancellationToken,
}

impl AsyncWrite for CancellingWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.bytes.extend_from_slice(bytes);
        self.cancel.cancel();
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn transfer_stops_after_the_first_written_block_when_cancelled()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = vec![9_u8; 131072];
    let digest = Sha256Digest::parse(format!("{:x}", Sha256::digest(&bytes)))?;
    let cancel = CancellationToken::new();
    let mut output = CancellingWriter {
        bytes: Vec::new(),
        cancel: cancel.clone(),
    };
    let mut input = Cursor::new(bytes);
    assert!(matches!(
        transfer(
            &mut input,
            &mut output,
            131072,
            &digest,
            &cancel,
            Instant::now() + Duration::from_secs(2)
        )
        .await,
        Err(PortError::Cancelled)
    ));
    assert_eq!(output.bytes.len(), 65536);
    Ok(())
}

#[tokio::test]
async fn expired_transfer_deadline_does_not_read_or_write() -> Result<(), Box<dyn std::error::Error>>
{
    let mut input = Cursor::new(vec![1_u8; 16]);
    let mut output = Vec::new();
    let digest = Sha256Digest::parse("a".repeat(64))?;
    assert!(
        transfer(
            &mut input,
            &mut output,
            16,
            &digest,
            &CancellationToken::new(),
            Instant::now() - Duration::from_secs(1)
        )
        .await
        .is_err()
    );
    assert_eq!(input.position(), 0);
    assert!(output.is_empty());
    Ok(())
}
