use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::error::PortError;

use super::mutations;
use super::queries;

pub struct SqliteArtifactIndex {
    pool: SqlitePool,
}

impl SqliteArtifactIndex {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl ArtifactIndex for SqliteArtifactIndex {
    async fn add(&self, project_id: &ProjectId, artifact: &Artifact) -> Result<(), PortError> {
        mutations::upsert_artifact(self.pool(), project_id, artifact).await
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, PortError> {
        queries::get_ready_artifact(self.pool(), id).await
    }

    async fn check_exists(&self, id: &ArtifactId) -> Result<bool, PortError> {
        queries::artifact_exists(self.pool(), id).await
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Artifact>, PortError> {
        queries::list_ready_by_project(self.pool(), project_id).await
    }

    async fn list_by_project_and_kind(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
    ) -> Result<Vec<Artifact>, PortError> {
        queries::list_ready_by_project_and_kind(self.pool(), project_id, kind).await
    }

    async fn delete(&self, id: &ArtifactId) -> Result<(), PortError> {
        mutations::delete_artifact(self.pool(), id).await
    }

    async fn update_state(
        &self,
        id: &ArtifactId,
        state: domain::media::ArtifactState,
        ready_at: Option<domain::chrono::DateTime<domain::chrono::Utc>>,
    ) -> Result<(), PortError> {
        mutations::update_artifact_state(self.pool(), id, state, ready_at).await
    }
}
