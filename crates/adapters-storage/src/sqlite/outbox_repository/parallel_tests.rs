#![allow(clippy::unwrap_used)]

use std::time::Duration;

use domain::outbox::{OutboxMessage, OutboxPayload};
use domain::project::ProjectId;
use ports::repository::OutboxRepository;
use sqlx::SqlitePool;

use super::SqliteOutboxRepository;

async fn insert(pool: &SqlitePool, group: &str) -> OutboxMessage {
    let message = OutboxMessage::new(OutboxPayload::DeleteProjectArtifactDir {
        project_id: ProjectId::new(),
    });
    let sequence: i64 = sqlx::query_scalar(
        "UPDATE outbox_sequence SET value = value + 1 WHERE singleton = 1 RETURNING value",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts,
          next_attempt_at, created_at, updated_at, sequence, conflict_group)
         VALUES (?, 'delete_project_artifact_dir', ?, 'pending', 0,
          strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), '2026-01-01T00:00:00Z',
          '2026-01-01T00:00:00Z', ?, ?)",
    )
    .bind(message.id.to_string())
    .bind(serde_json::to_string(&message.payload).unwrap())
    .bind(sequence)
    .bind(group)
    .execute(pool)
    .await
    .unwrap();
    message
}

#[tokio::test]
async fn equal_timestamps_keep_project_order_and_allow_other_projects() {
    let directory = tempfile::tempdir().unwrap();
    let pool = crate::sqlite::connect_sqlite(directory.path().join("outbox.sqlite"))
        .await
        .unwrap();
    let repo = SqliteOutboxRepository::new(pool.clone());
    let first = insert(&pool, "project:a").await;
    let second = insert(&pool, "project:a").await;
    let independent = insert(&pool, "project:b").await;

    let claims = repo
        .claim_ready(3, Duration::from_secs(300))
        .await
        .unwrap()
        .claims;
    assert_eq!(claims.len(), 2);
    assert!(claims.iter().any(|claim| claim.message.id == first.id));
    assert!(
        claims
            .iter()
            .any(|claim| claim.message.id == independent.id)
    );
    let other = claims
        .iter()
        .find(|claim| claim.message.id == independent.id)
        .unwrap();
    assert!(
        repo.mark_done_claim(&other.message.id, &other.token)
            .await
            .unwrap()
    );
    assert!(
        repo.claim_ready(3, Duration::from_secs(300))
            .await
            .unwrap()
            .claims
            .is_empty()
    );
    let earlier = claims
        .iter()
        .find(|claim| claim.message.id == first.id)
        .unwrap();
    assert!(
        repo.mark_done_claim(&earlier.message.id, &earlier.token)
            .await
            .unwrap()
    );
    let next = repo
        .claim_ready(3, Duration::from_secs(300))
        .await
        .unwrap()
        .claims;
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].message.id, second.id);
}

#[tokio::test]
async fn retry_and_dead_predecessors_block_only_their_group() {
    let directory = tempfile::tempdir().unwrap();
    let pool = crate::sqlite::connect_sqlite(directory.path().join("outbox.sqlite"))
        .await
        .unwrap();
    let repo = SqliteOutboxRepository::new(pool.clone());
    let first = insert(&pool, "project:a").await;
    let second = insert(&pool, "project:a").await;
    let other = insert(&pool, "project:b").await;
    let claims = repo
        .claim_ready(2, Duration::from_secs(300))
        .await
        .unwrap()
        .claims;
    let earlier = claims
        .iter()
        .find(|claim| claim.message.id == first.id)
        .unwrap();
    assert!(
        repo.mark_failed_claim(&first.id, &earlier.token, "retry")
            .await
            .unwrap()
    );
    let independent = claims
        .iter()
        .find(|claim| claim.message.id == other.id)
        .unwrap();
    assert!(
        repo.mark_done_claim(&other.id, &independent.token)
            .await
            .unwrap()
    );
    assert!(
        repo.claim_ready(2, Duration::from_secs(300))
            .await
            .unwrap()
            .claims
            .is_empty()
    );
    sqlx::query("UPDATE outbox_messages SET status = 'dead', updated_at = '2000-01-01T00:00:00Z' WHERE id = ?")
        .bind(first.id.to_string()).execute(&pool).await.unwrap();
    let pruned = repo
        .prune_terminal_rows(chrono::Utc::now(), chrono::Utc::now(), 10)
        .await
        .unwrap();
    assert_eq!(pruned.dead_deleted, 0);
    assert!(
        repo.claim_ready(2, Duration::from_secs(300))
            .await
            .unwrap()
            .claims
            .is_empty()
    );
    let status: String = sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = ?")
        .bind(second.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");
}

#[tokio::test]
async fn unique_attempt_token_fences_reclaimed_handler() {
    let directory = tempfile::tempdir().unwrap();
    let pool = crate::sqlite::connect_sqlite(directory.path().join("outbox.sqlite"))
        .await
        .unwrap();
    let repo = SqliteOutboxRepository::new(pool.clone());
    let message = insert(&pool, "project:a").await;
    let first = repo
        .claim_ready(1, Duration::from_secs(2))
        .await
        .unwrap()
        .claims
        .remove(0);
    assert!(repo.renew_claim(&message.id, &first.token).await.unwrap());
    sqlx::query("UPDATE outbox_messages SET locked_at = '2000-01-01T00:00:00Z' WHERE id = ?")
        .bind(message.id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    assert!(repo.renew_claim(&message.id, &first.token).await.unwrap());
    assert!(
        repo.claim_ready(1, Duration::from_secs(2))
            .await
            .unwrap()
            .claims
            .is_empty()
    );
    sqlx::query("UPDATE outbox_messages SET locked_at = '2000-01-01T00:00:00Z' WHERE id = ?")
        .bind(message.id.to_string())
        .execute(&pool)
        .await
        .unwrap();
    let second = repo
        .claim_ready(1, Duration::from_secs(2))
        .await
        .unwrap()
        .claims
        .remove(0);
    assert_ne!(first.token, second.token);
    assert!(
        !repo
            .mark_done_claim(&message.id, &first.token)
            .await
            .unwrap()
    );
    assert!(
        !repo
            .mark_failed_claim(&message.id, &first.token, "stale")
            .await
            .unwrap()
    );
    assert!(!repo.renew_claim(&message.id, &first.token).await.unwrap());
    assert!(
        repo.claim_is_current(&message.id, &second.token)
            .await
            .unwrap()
    );
    assert!(
        repo.mark_done_claim(&message.id, &second.token)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn legacy_unknown_group_serializes_with_every_project() {
    let directory = tempfile::tempdir().unwrap();
    let pool = crate::sqlite::connect_sqlite(directory.path().join("outbox.sqlite"))
        .await
        .unwrap();
    let repo = SqliteOutboxRepository::new(pool.clone());
    let legacy = insert(&pool, "*").await;
    let later = insert(&pool, "project:b").await;
    let first = repo
        .claim_ready(2, Duration::from_secs(300))
        .await
        .unwrap()
        .claims;
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].message.id, legacy.id);
    assert!(
        repo.claim_ready(2, Duration::from_secs(300))
            .await
            .unwrap()
            .claims
            .is_empty()
    );
    assert!(
        repo.mark_done_claim(&legacy.id, &first[0].token)
            .await
            .unwrap()
    );
    let next = repo
        .claim_ready(2, Duration::from_secs(300))
        .await
        .unwrap()
        .claims;
    assert_eq!(next[0].message.id, later.id);
}

#[tokio::test]
async fn two_workers_cannot_claim_the_same_attempt() {
    let directory = tempfile::tempdir().unwrap();
    let pool = crate::sqlite::connect_sqlite(directory.path().join("outbox.sqlite"))
        .await
        .unwrap();
    let message = insert(&pool, "project:a").await;
    let left = SqliteOutboxRepository::new(pool.clone());
    let right = left.clone();
    let (one, two) = tokio::join!(
        left.claim_ready(1, Duration::from_secs(300)),
        right.claim_ready(1, Duration::from_secs(300)),
    );
    let claimed: Vec<_> = one
        .unwrap()
        .claims
        .into_iter()
        .chain(two.unwrap().claims)
        .collect();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].message.id, message.id);
}
