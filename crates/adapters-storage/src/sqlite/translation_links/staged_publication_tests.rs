#![allow(clippy::unwrap_used)]

use super::SqliteTranslationLinkStore;
use crate::sqlite::connect_sqlite;
use chrono::Utc;
use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::StagedArtifact;
use ports::translation_links::{PublicationIntent, TranslationIntent, TranslationLinkStore};
use uuid::Uuid;

fn staged(project_id: &ProjectId, artifact_id: &ArtifactId) -> StagedArtifact {
    let final_key = format!("{project_id}/translated/{artifact_id}.srt");
    StagedArtifact {
        artifact: Artifact {
            id: artifact_id.clone(),
            kind: ArtifactKind::TranslatedTranscript,
            location: ArtifactLocation::StorageKey(final_key.clone()),
            size_bytes: Some(42),
            state: ArtifactState::PendingFinalize,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ready_at: None,
        },
        staging_key: format!(".staging/{artifact_id}.srt"),
        final_key,
        size_bytes: 42,
    }
}

#[tokio::test]
async fn artifact_publication_and_outbox_commit_together_and_retry() {
    let directory = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(directory.path().join("auralis.sqlite"))
        .await
        .unwrap();
    let project_id = ProjectId::new();
    let source_id = ArtifactId::new();
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'Test', 'Draft', 'now', 'now')")
        .bind(project_id.to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now')")
        .bind(source_id.to_string()).bind(project_id.to_string()).execute(&pool).await.unwrap();
    let intent = TranslationIntent {
        translation_id: Uuid::new_v4(),
        project_id: project_id.clone(),
        source_artifact_id: source_id,
        source_sha256: "a".repeat(64),
        source_format: "srt".into(),
        source_language: "zh".into(),
        target_language: "ru".into(),
        run_id: Uuid::new_v4(),
        created_at: Utc::now(),
    };
    let store = SqliteTranslationLinkStore::new(pool.clone());
    store.begin(intent.clone()).await.unwrap();
    let artifact_id = ArtifactId::new();
    let publication = PublicationIntent {
        translation_id: intent.translation_id,
        result_id: Uuid::new_v4(),
        result_revision: 1,
        run_id: intent.run_id,
        artifact_id: artifact_id.clone(),
        source_sha256: intent.source_sha256.clone(),
        expected_link_revision: 1,
        needs_review: true,
        created_at: Utc::now(),
    };
    store
        .commit_staged_publication(publication.clone(), staged(&project_id, &artifact_id))
        .await
        .unwrap();
    store
        .commit_staged_publication(publication.clone(), staged(&project_id, &artifact_id))
        .await
        .unwrap();
    let artifact_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts WHERE id = ?")
        .bind(artifact_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    let publication_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM translation_publications WHERE translation_id = ? AND result_id = ?",
    )
    .bind(intent.translation_id.to_string())
    .bind(publication.result_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    let outbox_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages WHERE deduplication_key = ?")
            .bind(format!(
                "translation-publication:{}:{}",
                intent.translation_id, publication.result_id
            ))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((artifact_count, publication_count, outbox_count), (1, 1, 1));

    sqlx::query("UPDATE translation_publications SET state = 'failed' WHERE translation_id = ? AND result_id = ?")
        .bind(intent.translation_id.to_string())
        .bind(publication.result_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .commit_staged_publication(publication.clone(), staged(&project_id, &artifact_id))
            .await,
        Err(PortError::Conflict { .. })
    ));
    sqlx::query("UPDATE translation_publications SET state = 'pending' WHERE translation_id = ? AND result_id = ?")
        .bind(intent.translation_id.to_string())
        .bind(publication.result_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE outbox_messages SET status = 'dead' WHERE deduplication_key = ?")
        .bind(format!(
            "translation-publication:{}:{}",
            intent.translation_id, publication.result_id
        ))
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .commit_staged_publication(publication.clone(), staged(&project_id, &artifact_id))
            .await,
        Err(PortError::Conflict { .. })
    ));
    sqlx::query("UPDATE outbox_messages SET status = 'pending' WHERE deduplication_key = ?")
        .bind(format!(
            "translation-publication:{}:{}",
            intent.translation_id, publication.result_id
        ))
        .execute(&pool)
        .await
        .unwrap();

    let stale_artifact = ArtifactId::new();
    let stale = PublicationIntent {
        result_id: Uuid::new_v4(),
        artifact_id: stale_artifact.clone(),
        expected_link_revision: 9,
        ..publication.clone()
    };
    assert!(matches!(
        store
            .commit_staged_publication(stale.clone(), staged(&project_id, &stale_artifact))
            .await,
        Err(PortError::Conflict { .. })
    ));
    let rolled_back: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts WHERE id = ?")
        .bind(stale_artifact.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rolled_back, 0);
    let publication_rolled_back: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM translation_publications WHERE translation_id = ? AND result_id = ?",
    )
    .bind(stale.translation_id.to_string())
    .bind(stale.result_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(publication_rolled_back, 0);

    sqlx::query("UPDATE artifacts SET state = 'ready', ready_at = 'now' WHERE id = ?")
        .bind(artifact_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    store
        .finalize_ready_publication(intent.translation_id, publication.result_id, Utc::now())
        .await
        .unwrap();
    store
        .commit_staged_publication(publication, staged(&project_id, &artifact_id))
        .await
        .unwrap();
}

#[tokio::test]
async fn newer_edit_wins_when_both_publications_are_waiting_for_finalization() {
    let directory = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(directory.path().join("auralis.sqlite"))
        .await
        .unwrap();
    let project_id = ProjectId::new();
    let source_id = ArtifactId::new();
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'Test', 'Draft', 'now', 'now')")
        .bind(project_id.to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now')")
        .bind(source_id.to_string()).bind(project_id.to_string()).execute(&pool).await.unwrap();
    let intent = TranslationIntent {
        translation_id: Uuid::new_v4(),
        project_id: project_id.clone(),
        source_artifact_id: source_id,
        source_sha256: "a".repeat(64),
        source_format: "srt".into(),
        source_language: "zh".into(),
        target_language: "ru".into(),
        run_id: Uuid::new_v4(),
        created_at: Utc::now(),
    };
    let store = SqliteTranslationLinkStore::new(pool.clone());
    store.begin(intent.clone()).await.unwrap();
    let first_artifact = ArtifactId::new();
    let first = PublicationIntent {
        translation_id: intent.translation_id,
        result_id: Uuid::new_v4(),
        result_revision: 1,
        run_id: intent.run_id,
        artifact_id: first_artifact.clone(),
        source_sha256: intent.source_sha256.clone(),
        expected_link_revision: 1,
        needs_review: true,
        created_at: Utc::now(),
    };
    store
        .commit_staged_publication(first.clone(), staged(&project_id, &first_artifact))
        .await
        .unwrap();
    let edited_artifact = ArtifactId::new();
    let edited = PublicationIntent {
        result_id: Uuid::new_v4(),
        result_revision: 2,
        artifact_id: edited_artifact.clone(),
        ..first.clone()
    };
    store
        .commit_staged_publication(edited.clone(), staged(&project_id, &edited_artifact))
        .await
        .unwrap();

    sqlx::query("UPDATE artifacts SET state = 'ready', ready_at = 'now' WHERE id IN (?, ?)")
        .bind(first_artifact.to_string())
        .bind(edited_artifact.to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(store.list_ready_publications(10).await.unwrap().len(), 1);
    assert_eq!(
        store.list_ready_publications(10).await.unwrap()[0].result_id,
        edited.result_id
    );
    assert!(matches!(
        store
            .finalize_ready_publication(intent.translation_id, first.result_id, Utc::now())
            .await,
        Err(PortError::Conflict { .. })
    ));
    sqlx::query("UPDATE translation_publications SET state = 'failed' WHERE result_id = ?")
        .bind(edited.result_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.list_ready_publications(10).await.unwrap().is_empty());
    sqlx::query("UPDATE translation_publications SET state = 'pending' WHERE result_id = ?")
        .bind(edited.result_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    let selected = store
        .finalize_ready_publication(intent.translation_id, edited.result_id, Utc::now())
        .await
        .unwrap();
    assert_eq!(selected.selected_result_id, Some(edited.result_id));
    assert_eq!(selected.output_artifact_id, Some(edited_artifact));
    let first_stored = store
        .get_publication(intent.translation_id, first.result_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first_stored.result_revision, 1);
    let edited_stored = store
        .get_publication(intent.translation_id, edited.result_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(edited_stored.result_revision, 2);
}
