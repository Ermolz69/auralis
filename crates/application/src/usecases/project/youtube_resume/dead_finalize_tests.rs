#![allow(clippy::unwrap_used)]

use super::{
    fixture::Fixture,
    retention_tests::{drain, maintenance},
    tests::kill_at,
};
use adapters_storage::sqlite::SqliteOutboxRepository;
use ports::{
    repository::OutboxRepository, storage::ArtifactStore, youtube_import::YoutubeImportJournal,
};

#[tokio::test]
async fn youtube_dead_finalize_survives_two_retention_cycles_and_can_be_retried() {
    let root = tempfile::tempdir().unwrap();
    kill_at(root.path(), "stage-after-checkpoint");
    let fixture = Fixture::open(root.path()).await;
    let session = fixture.journal.list().await.unwrap().remove(0);
    let stage = session.write.as_ref().unwrap();
    fixture.usecase().resume(&session.project.id).await.unwrap();
    assert!(fixture.journal.list().await.unwrap().is_empty());
    let repo = SqliteOutboxRepository::new(fixture.pool.clone());
    let id: String = sqlx::query_scalar(
        "SELECT id FROM outbox_messages WHERE kind = 'finalize_staged_artifact'",
    )
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    for _ in 0..5 {
        sqlx::query("UPDATE outbox_messages SET status = 'processing' WHERE id = ?")
            .bind(&id)
            .execute(&fixture.pool)
            .await
            .unwrap();
        repo.mark_failed(&id.parse().unwrap(), "FINALIZE_IO")
            .await
            .unwrap();
    }
    sqlx::query("UPDATE outbox_messages SET updated_at = '2000-01-01T00:00:00Z' WHERE id = ?")
        .bind(&id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    for _ in 0..2 {
        maintenance(&fixture).await;
        assert!(
            fixture
                .store
                .verify_staging(&stage.staging_key, stage.artifact.size_bytes.unwrap())
                .await
                .unwrap()
        );
        let status: String = sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = ?")
            .bind(&id)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(status, "dead");
    }
    let recovery = crate::usecases::artifact::recovery::ArtifactRecoveryUseCase::new(
        std::sync::Arc::new(repo),
    );
    assert_eq!(
        recovery.list().await.unwrap(),
        vec![session.project.id.clone()]
    );
    recovery.retry(&session.project.id).await.unwrap();
    recovery.retry(&session.project.id).await.unwrap();
    assert!(recovery.list().await.unwrap().is_empty());
    drain(&fixture).await;
    drain(&fixture).await;
    let state: String = sqlx::query_scalar("SELECT state FROM artifacts WHERE id = ?")
        .bind(stage.artifact.id.to_string())
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(state, "ready");
    assert_eq!(
        std::fs::read(root.path().join("network-completions")).unwrap(),
        b"1"
    );
    fixture.pool.close().await;
}
