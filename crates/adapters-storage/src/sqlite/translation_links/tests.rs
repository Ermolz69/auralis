#![allow(clippy::unwrap_used)]

use super::SqliteTranslationLinkStore;
use crate::sqlite::connect_sqlite;
use chrono::Utc;
use domain::media::ArtifactId;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::translation_links::{
    PublicationIntent, StartTranslationRun, TranslationIntent, TranslationLinkStore,
};
use sqlx::SqlitePool;
use uuid::Uuid;

async fn seed_project(pool: &SqlitePool, project_id: &ProjectId, source_id: &ArtifactId) {
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'Test', 'Draft', 'now', 'now')")
        .bind(project_id.to_string()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, 'ready', 'now', 'now')")
        .bind(source_id.to_string()).bind(project_id.to_string()).bind(format!("source-{source_id}.srt")).execute(pool).await.unwrap();
}

#[tokio::test]
async fn project_and_active_lists_keep_links_visible_across_restart() {
    let directory = tempfile::tempdir().unwrap();
    let db_path = directory.path().join("auralis.sqlite");
    let pool = connect_sqlite(&db_path).await.unwrap();
    let first_project = ProjectId::new();
    let second_project = ProjectId::new();
    let first_source = ArtifactId::new();
    let second_source = ArtifactId::new();
    seed_project(&pool, &first_project, &first_source).await;
    seed_project(&pool, &second_project, &second_source).await;
    let first = intent(first_project.clone(), first_source);
    let second = intent(second_project.clone(), second_source);
    let store = SqliteTranslationLinkStore::new(pool.clone());
    store.begin(first.clone()).await.unwrap();
    store.begin(second.clone()).await.unwrap();
    assert_eq!(
        store.list_by_project(&first_project).await.unwrap()[0].translation_id,
        first.translation_id
    );
    assert_eq!(
        store.list_by_project(&first_project).await.unwrap().len(),
        1
    );
    assert_eq!(
        store.list_by_project(&second_project).await.unwrap().len(),
        1
    );
    assert_eq!(store.list_active().await.unwrap().len(), 2);
    pool.close().await;

    let reopened = connect_sqlite(&db_path).await.unwrap();
    let store = SqliteTranslationLinkStore::new(reopened);
    let active = store.list_active().await.unwrap();
    assert_eq!(active.len(), 2);
    assert!(active.iter().any(|link| {
        link.translation_id == first.translation_id && link.active_run_id == Some(first.run_id)
    }));
    assert!(active.iter().any(|link| {
        link.translation_id == second.translation_id && link.active_run_id == Some(second.run_id)
    }));
}

fn intent(project_id: ProjectId, source_id: ArtifactId) -> TranslationIntent {
    TranslationIntent {
        translation_id: Uuid::new_v4(),
        project_id,
        source_artifact_id: source_id,
        source_sha256: "a".repeat(64),
        source_format: "srt".into(),
        source_language: "zh".into(),
        target_language: "ru".into(),
        run_id: Uuid::new_v4(),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn intent_is_durable_idempotent_and_source_bound() {
    let directory = tempfile::tempdir().unwrap();
    let db_path = directory.path().join("auralis.sqlite");
    let pool = connect_sqlite(&db_path).await.unwrap();
    let project_id = ProjectId::new();
    let source_id = ArtifactId::new();
    seed_project(&pool, &project_id, &source_id).await;
    let store = SqliteTranslationLinkStore::new(pool.clone());
    let request = intent(project_id, source_id);
    let first = store.begin(request.clone()).await.unwrap();
    assert_eq!(first.active_run_id, Some(request.run_id));
    assert_eq!(first.revision, 1);
    assert_eq!(store.begin(request.clone()).await.unwrap(), first);
    let changed = TranslationIntent {
        source_sha256: "b".repeat(64),
        ..request.clone()
    };
    assert!(matches!(
        store.begin(changed).await,
        Err(PortError::Conflict { .. })
    ));
    pool.close().await;

    let reopened = connect_sqlite(&db_path).await.unwrap();
    let stored = SqliteTranslationLinkStore::new(reopened)
        .get(request.translation_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored, first);
}

#[tokio::test]
async fn result_is_selected_only_after_ready_artifact_and_matching_revision() {
    let directory = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(directory.path().join("auralis.sqlite"))
        .await
        .unwrap();
    let project_id = ProjectId::new();
    let source_id = ArtifactId::new();
    seed_project(&pool, &project_id, &source_id).await;
    let request = intent(project_id.clone(), source_id);
    let store = SqliteTranslationLinkStore::new(pool.clone());
    store.begin(request.clone()).await.unwrap();
    let output_id = ArtifactId::new();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'TranslatedTranscript', 'output.srt', 'pending_finalize', 'now', 'now')")
        .bind(output_id.to_string()).bind(project_id.to_string()).execute(&pool).await.unwrap();
    let publication = PublicationIntent {
        translation_id: request.translation_id,
        result_id: Uuid::new_v4(),
        run_id: request.run_id,
        artifact_id: output_id.clone(),
        source_sha256: request.source_sha256.clone(),
        expected_link_revision: 1,
        needs_review: true,
        created_at: Utc::now(),
    };
    store.record_publication(publication.clone()).await.unwrap();
    store.record_publication(publication.clone()).await.unwrap();
    assert!(matches!(
        store
            .select_ready_result(
                publication.translation_id,
                publication.result_id,
                Utc::now()
            )
            .await,
        Err(PortError::Conflict { .. })
    ));
    assert!(
        store
            .get(publication.translation_id)
            .await
            .unwrap()
            .unwrap()
            .selected_result_id
            .is_none()
    );
    sqlx::query("UPDATE artifacts SET state = 'ready', ready_at = 'now' WHERE id = ?")
        .bind(output_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE translation_publications SET state = 'failed' WHERE translation_id = ? AND result_id = ?")
        .bind(publication.translation_id.to_string())
        .bind(publication.result_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .select_ready_result(
                publication.translation_id,
                publication.result_id,
                Utc::now()
            )
            .await,
        Err(PortError::Conflict { .. })
    ));
    sqlx::query("UPDATE translation_publications SET state = 'pending' WHERE translation_id = ? AND result_id = ?")
        .bind(publication.translation_id.to_string())
        .bind(publication.result_id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    let selected = store
        .select_ready_result(
            publication.translation_id,
            publication.result_id,
            Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(selected.selected_result_id, Some(publication.result_id));
    assert_eq!(selected.output_artifact_id, Some(output_id));
    assert_eq!(selected.active_run_id, None);
    assert_eq!(selected.revision, 2);
    assert!(store.list_active().await.unwrap().is_empty());
    assert_eq!(
        store
            .select_ready_result(
                publication.translation_id,
                publication.result_id,
                Utc::now()
            )
            .await
            .unwrap(),
        selected
    );
    let newer = PublicationIntent {
        result_id: Uuid::new_v4(),
        ..publication
    };
    assert!(matches!(
        store.record_publication(newer).await,
        Err(PortError::Conflict { .. })
    ));
    let next = StartTranslationRun {
        translation_id: request.translation_id,
        run_id: Uuid::new_v4(),
        expected_active_run_id: None,
        expected_link_revision: selected.revision,
        updated_at: Utc::now(),
    };
    let running = store.start_run(next.clone()).await.unwrap();
    assert_eq!(store.list_active().await.unwrap().len(), 1);
    assert_eq!(running.active_run_id, Some(next.run_id));
    assert_eq!(running.selected_result_id, selected.selected_result_id);
    assert_eq!(running.output_artifact_id, selected.output_artifact_id);
    assert_eq!(running.revision, selected.revision + 1);
    assert_eq!(store.start_run(next).await.unwrap(), running);
}
