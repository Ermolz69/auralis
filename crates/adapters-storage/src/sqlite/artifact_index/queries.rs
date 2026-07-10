use sqlx::SqlitePool;

use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;

use super::mapper::row_to_artifact;
use super::row::ArtifactRow;
use super::serialization::artifact_kind_to_db;

pub async fn get_ready_artifact(
    pool: &SqlitePool,
    id: &ArtifactId,
) -> Result<Option<Artifact>, PortError> {
    let row = sqlx::query_as::<_, ArtifactRow>(
        r#"
        SELECT 
            id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
        FROM artifacts
        WHERE id = ? AND state = 'ready'
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to fetch artifact: {}", e),
    })?;

    row.map(row_to_artifact).transpose()
}

pub async fn artifact_exists(pool: &SqlitePool, id: &ArtifactId) -> Result<bool, PortError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ?)
        "#,
    )
    .bind(id.to_string())
    .fetch_one(pool)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to check artifact existence: {}", e),
    })?;

    Ok(exists)
}

pub async fn list_ready_by_project(
    pool: &SqlitePool,
    project_id: &ProjectId,
) -> Result<Vec<Artifact>, PortError> {
    let rows = sqlx::query_as::<_, ArtifactRow>(
        r#"
        SELECT 
            id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
        FROM artifacts
        WHERE project_id = ? AND state = 'ready'
        ORDER BY created_at ASC
        "#,
    )
    .bind(project_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to list artifacts by project: {}", e),
    })?;

    let mut artifacts = Vec::new();
    for row in rows {
        artifacts.push(row_to_artifact(row)?);
    }
    Ok(artifacts)
}

pub async fn list_ready_by_project_and_kind(
    pool: &SqlitePool,
    project_id: &ProjectId,
    kind: ArtifactKind,
) -> Result<Vec<Artifact>, PortError> {
    let kind_str = artifact_kind_to_db(&kind)?;

    let rows = sqlx::query_as::<_, ArtifactRow>(
        r#"
        SELECT 
            id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
        FROM artifacts
        WHERE project_id = ? AND kind = ? AND state = 'ready'
        ORDER BY created_at ASC
        "#,
    )
    .bind(project_id.to_string())
    .bind(kind_str)
    .fetch_all(pool)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to list artifacts by project and kind: {}", e),
    })?;

    let mut artifacts = Vec::new();
    for row in rows {
        artifacts.push(row_to_artifact(row)?);
    }
    Ok(artifacts)
}
