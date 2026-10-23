use super::*;
use domain::media::{ArtifactKind, ArtifactLocation};

#[tokio::test]
async fn layout_is_a_managed_metadata_artifact_and_stays_hidden_until_outbox_finishes()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, cases, engine, _) = fixture().await?;
    engine.hold.store(false, Ordering::SeqCst);
    engine.include_layout.store(true, Ordering::SeqCst);
    let review = cases
        .review(fixture.project_id.clone(), source(), true)
        .await?;
    let prepared = cases
        .prepare(fixture.project_id.clone(), request(&review))
        .await?;
    cases
        .start(fixture.project_id.clone(), prepared.run_id, false)
        .await?;
    let pending = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(publication) = cases.store.publication(prepared.run_id).await? {
                return Ok::<_, PortError>(publication);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await??;
    assert_eq!(pending.files.len(), 4);
    assert!(!pending.ready);
    assert!(
        cases
            .state(fixture.project_id.clone(), prepared.run_id)
            .await?
            .result
            .is_none()
    );
    fixture.flush().await?;
    let ready = cases
        .state(fixture.project_id.clone(), prepared.run_id)
        .await?
        .result
        .ok_or("Ready result missing")?;
    let metadata = ready
        .files
        .iter()
        .find(|file| file.kind == "audio_layout")
        .ok_or("Layout metadata missing")?;
    let artifact = cases
        .index
        .get(&metadata.artifact_id)
        .await?
        .ok_or("Managed layout missing")?;
    assert_eq!(artifact.kind, ArtifactKind::SpeechManifest);
    let ArtifactLocation::StorageKey(key) = artifact.location else {
        return Err("Layout is not managed".into());
    };
    assert!(key.ends_with(".json"));
    assert!(
        cases
            .artifacts
            .verify_storage_digest(&key, metadata.size_bytes, &metadata.sha256)
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn cancelled_layout_publication_discards_every_pending_file_before_cached_resume()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, cases, engine, manager) = fixture().await?;
    engine.hold.store(false, Ordering::SeqCst);
    engine.include_layout.store(true, Ordering::SeqCst);
    let review = cases
        .review(fixture.project_id.clone(), source(), true)
        .await?;
    let prepared = cases
        .prepare(fixture.project_id.clone(), request(&review))
        .await?;
    let started = cases
        .start(fixture.project_id.clone(), prepared.run_id, false)
        .await?;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while cases.store.publication(prepared.run_id).await?.is_none() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<_, PortError>(())
    })
    .await??;
    manager
        .cancel_job(&started.job.ok_or("Missing job")?.id)
        .await?;
    assert!(cases.store.publication(prepared.run_id).await?.is_none());
    assert!(
        cases
            .state(fixture.project_id.clone(), prepared.run_id)
            .await?
            .result
            .is_none()
    );
    cases
        .start(fixture.project_id.clone(), prepared.run_id, true)
        .await?;
    let ready = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            fixture.flush().await?;
            if let Some(publication) = cases
                .state(fixture.project_id.clone(), prepared.run_id)
                .await?
                .result
            {
                return Ok::<_, Box<dyn std::error::Error>>(publication);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await??;
    assert_eq!(
        ready
            .files
            .iter()
            .filter(|file| file.kind == "audio_layout")
            .count(),
        1
    );
    assert!(ready.ready);
    Ok(())
}
