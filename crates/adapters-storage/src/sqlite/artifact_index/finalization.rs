use async_trait::async_trait;
use domain::{media::ArtifactId, project::ProjectId};
use ports::{
    artifact_finalization::{ArtifactFinalizationLookup, FinalizationMetadata},
    error::PortError,
};

use super::{SqliteArtifactIndex, mapper::row_to_artifact, row::ArtifactRow};
use crate::sqlite::helpers::map_sqlite_error;

#[async_trait]
impl ArtifactFinalizationLookup for SqliteArtifactIndex {
    async fn get_for_finalization(
        &self,
        project_id: &ProjectId,
        artifact_id: &ArtifactId,
    ) -> Result<FinalizationMetadata, PortError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| map_sqlite_error("finalization_lookup", e))?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?)")
            .bind(project_id.to_string())
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| map_sqlite_error("finalization_owner", e))?;
        if !exists {
            return Ok(FinalizationMetadata::ProjectDeleted);
        }
        let row = sqlx::query_as::<_, ArtifactRow>(
            "SELECT * FROM artifacts WHERE id = ? AND project_id = ?",
        )
        .bind(artifact_id.to_string())
        .bind(project_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| map_sqlite_error("finalization_artifact", e))?
        .ok_or_else(|| PortError::NotFound {
            resource: "Finalization artifact".into(),
        })?;
        Ok(FinalizationMetadata::Artifact(row_to_artifact(row)?))
    }
}
