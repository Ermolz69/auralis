use super::*;
#[path = "video_source.rs"]
mod video_source;

#[tokio::test]
async fn video_publication_hides_all_outputs_until_the_fourth_file_is_ready()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AssemblySource::new().await?;
    let store = SqliteAudioAssemblyStore::new(fixture.pool.clone());
    let mut plan = fixture.plan()?;
    plan.video = Some(video_source::video_source(&fixture).await?);
    let intent = store.register(&fixture.with_plan(plan)?).await?;
    let attempt = Uuid::new_v4();
    let mut job = store
        .create_pending(
            intent.assembly_id,
            attempt,
            Job::new(
                fixture.project_id.clone(),
                "Video fixture".into(),
                JobKind::AudioAssembly,
            ),
        )
        .await?;
    let revision = job.revision();
    job.start()?;
    store.update(&job, attempt, revision).await?;
    let (mut publication, mut staged) = output(&fixture, &intent, attempt).await?;
    for (role, kind) in [
        ("video", ArtifactKind::FinalVideo),
        ("video_manifest", ArtifactKind::SpeechManifest),
    ] {
        let file = fixture.directory.path().join(role);
        std::fs::write(&file, role.as_bytes())?;
        let hash = source::digest(role.as_bytes());
        let artifact = fixture
            .files
            .stage_verified_copy(
                VerifiedArtifactCopy {
                    project_id: fixture.project_id.clone(),
                    kind,
                    source_path: file,
                    filename_hint: None,
                    expected_sha256: Sha256Digest::parse(hash.clone())?,
                    max_bytes: 1024,
                },
                CancellationToken::new(),
            )
            .await?;
        publication.files.push(SpeechPublishedFile {
            artifact_id: artifact.artifact.id.clone(),
            kind: role.into(),
            sha256: hash,
            size_bytes: artifact.size_bytes,
        });
        staged.push(artifact);
    }
    assert!(
        store
            .publish(&job, &publication, &staged[..3])
            .await
            .is_err()
    );
    assert!(store.publication(intent.assembly_id).await?.is_none());
    store.publish(&job, &publication, &staged).await?;
    for file in &staged[..3] {
        finalize(&fixture, file).await?;
    }
    assert!(
        store
            .complete_publication(intent.assembly_id)
            .await?
            .is_none()
    );
    assert!(
        fixture
            .index
            .list_by_project_and_kind(&fixture.project_id, ArtifactKind::FinalVideo)
            .await?
            .is_empty()
    );
    finalize(&fixture, &staged[3]).await?;
    assert!(
        store
            .complete_publication(intent.assembly_id)
            .await?
            .is_some()
    );
    assert_eq!(
        fixture
            .index
            .list_by_project_and_kind(&fixture.project_id, ArtifactKind::FinalVideo)
            .await?
            .len(),
        1
    );
    let retained = store
        .publication(intent.assembly_id)
        .await?
        .ok_or("Missing video publication")?;
    assert!(retained.ready);
    assert_eq!(retained.files.len(), 4);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&fixture.pool)
            .await?
            .is_empty()
    );
    fixture.pool.close().await;
    let reopened = connect_sqlite(&fixture.db_path).await?;
    let reloaded = SqliteAudioAssemblyStore::new(reopened.clone())
        .publication(intent.assembly_id)
        .await?
        .ok_or("Lost video publication")?;
    assert!(reloaded.ready);
    assert_eq!(reloaded.files.len(), 4);
    reopened.close().await;
    Ok(())
}
