#![allow(clippy::unwrap_used)]
use super::setup_db;
use crate::sqlite::unit_of_work::SqliteStorageUnitOfWork;

use domain::media::{Artifact, ArtifactId};
use domain::project::Project;
use ports::error::PortError;
use ports::transaction::{CommitManagedSourceImport, StorageUnitOfWork};

async fn insert_draft_project(pool: &sqlx::SqlitePool, project: &Project) {
    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(project.id().to_string())
    .bind(project.title())
    .bind("Draft")
    .bind(project.created_at().to_rfc3339())
    .bind(project.updated_at().to_rfc3339())
    .execute(pool)
    .await
    .unwrap();
}

fn managed_source_import_command(
    mut project: Project,
    artifact_id: ArtifactId,
    original_updated_at: chrono::DateTime<chrono::Utc>,
) -> CommitManagedSourceImport {
    let final_key = format!("{}/source-video/{}.mp4", project.id(), artifact_id);
    let staging_key = format!(".staging/uuid/{}.mp4", artifact_id);

    let artifact = Artifact {
        id: artifact_id.clone(),
        kind: domain::media::ArtifactKind::SourceVideo,
        location: domain::media::ArtifactLocation::StorageKey(final_key.clone()),
        size_bytes: Some(1024),
        state: domain::media::ArtifactState::PendingFinalize,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        ready_at: None,
    };

    project
        .import_source(
            domain::media::MediaSource::ManagedLocalFile {
                artifact_id: artifact.id.clone(),
                original_filename: "test.mp4".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();

    CommitManagedSourceImport {
        project,
        artifact,
        staging_key,
        final_key,
        original_updated_at,
    }
}

#[tokio::test]
async fn managed_source_import_artifact_id_conflict_rolls_back_project() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());

    let existing_project = Project::new("Existing Artifact Owner".to_string());
    insert_draft_project(&pool, &existing_project).await;

    let import_project = Project::new("Import Target".to_string());
    let original_updated_at = import_project.updated_at();
    insert_draft_project(&pool, &import_project).await;

    let artifact_id = ArtifactId::new();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'StorageKey', ?, 1024, 'pending_finalize', ?, ?)")
        .bind(artifact_id.to_string())
        .bind(existing_project.id().to_string())
        .bind(format!("{}/source-video/{}.mp4", existing_project.id(), artifact_id))
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

    let cmd = managed_source_import_command(
        import_project.clone(),
        artifact_id.clone(),
        original_updated_at,
    );

    let err = uow.commit_managed_source_import(cmd).await.unwrap_err();
    assert!(matches!(
        err,
        PortError::Storage { .. } | PortError::Conflict { .. }
    ));

    let status: String = sqlx::query_scalar("SELECT status FROM projects WHERE id = ?")
        .bind(import_project.id().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Draft");

    let artifact_owner: String =
        sqlx::query_scalar("SELECT project_id FROM artifacts WHERE id = ?")
            .bind(artifact_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(artifact_owner, existing_project.id().to_string());

    let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_count, 0);
}

#[tokio::test]
async fn managed_source_import_outbox_failure_rolls_back_project_and_artifact() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());

    let project = Project::new("Outbox Failure".to_string());
    let original_updated_at = project.updated_at();
    insert_draft_project(&pool, &project).await;

    sqlx::query(
        "CREATE TRIGGER fail_outbox_insert BEFORE INSERT ON outbox_messages BEGIN SELECT RAISE(ABORT, 'forced outbox failure'); END;",
    )
    .execute(&pool)
    .await
    .unwrap();

    let artifact_id = ArtifactId::new();
    let cmd =
        managed_source_import_command(project.clone(), artifact_id.clone(), original_updated_at);

    let err = uow.commit_managed_source_import(cmd).await.unwrap_err();
    assert!(matches!(
        err,
        PortError::Storage { .. } | PortError::Busy { .. }
    ));

    let status: String = sqlx::query_scalar("SELECT status FROM projects WHERE id = ?")
        .bind(project.id().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Draft");

    let artifact_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts WHERE id = ?")
        .bind(artifact_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(artifact_count, 0);

    let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_count, 0);
}
