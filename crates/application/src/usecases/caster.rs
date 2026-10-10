use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::caster::{
    CastResult, CasterPort, LogicalCast, ManualSelection, ReferenceDetails, VoiceCatalogPage,
    VoiceCompatibility, VoiceDetails,
};
use ports::error::PortError;
use ports::repository::ProjectRepository;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::ApplicationError;
use crate::usecases::project::lifecycle::ProjectLifecycleLocks;

pub struct CasterUseCases {
    projects: Arc<dyn ProjectRepository>,
    artifacts: Arc<dyn ArtifactIndex>,
    caster: Arc<dyn CasterPort>,
    locks: Arc<ProjectLifecycleLocks>,
}

impl CasterUseCases {
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        artifacts: Arc<dyn ArtifactIndex>,
        caster: Arc<dyn CasterPort>,
        locks: Arc<ProjectLifecycleLocks>,
    ) -> Self {
        Self {
            projects,
            artifacts,
            caster,
            locks,
        }
    }

    async fn ensure_project(&self, project: &ProjectId) -> Result<(), ApplicationError> {
        self.projects
            .get(project)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project.clone()))?;
        Ok(())
    }

    async fn register_revision(
        &self,
        project: &ProjectId,
        result: &CastResult,
    ) -> Result<(), ApplicationError> {
        let revision = &result.revision_sha256;
        if revision.len() != 64
            || !revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(PortError::InvalidSource {
                message: "Caster revision digest is invalid".into(),
            }
            .into());
        }
        let key = format!("{project}/casting/revisions/{revision}.json");
        let digest = Sha256::digest(format!("auralis-manual-cast:{project}:{revision}").as_bytes());
        let mut id_bytes = [0_u8; 16];
        id_bytes.copy_from_slice(&digest[..16]);
        id_bytes[6] = (id_bytes[6] & 0x0f) | 0x80;
        id_bytes[8] = (id_bytes[8] & 0x3f) | 0x80;
        let id = ArtifactId(Uuid::from_bytes(id_bytes));
        if let Some(existing) = self.artifacts.get(&id).await? {
            if existing.kind != ArtifactKind::ManualCast
                || existing.location != ArtifactLocation::StorageKey(key)
                || existing.state != ArtifactState::Ready
            {
                return Err(PortError::Conflict {
                    resource: "manual_cast_artifact".into(),
                    message: "Registered cast artifact differs from the verified revision".into(),
                }
                .into());
            }
            return Ok(());
        }
        let now = chrono::Utc::now();
        self.artifacts
            .add(
                project,
                &Artifact {
                    id,
                    kind: ArtifactKind::ManualCast,
                    location: ArtifactLocation::StorageKey(key),
                    size_bytes: None,
                    state: ArtifactState::Ready,
                    created_at: now,
                    updated_at: now,
                    ready_at: Some(now),
                },
            )
            .await?;
        Ok(())
    }

    pub async fn catalog_page(
        &self,
        project: &ProjectId,
        after: Option<String>,
        limit: u32,
    ) -> Result<VoiceCatalogPage, ApplicationError> {
        self.ensure_project(project).await?;
        Ok(self.caster.catalog_page(project, after, limit).await?)
    }

    pub async fn inspect_voice(
        &self,
        project: &ProjectId,
        voice_id: String,
        after_reference: Option<String>,
        limit: u32,
    ) -> Result<VoiceDetails, ApplicationError> {
        self.ensure_project(project).await?;
        Ok(self
            .caster
            .inspect_voice(project, voice_id, after_reference, limit)
            .await?)
    }

    pub async fn inspect_reference(
        &self,
        project: &ProjectId,
        voice_id: String,
        reference_id: String,
    ) -> Result<ReferenceDetails, ApplicationError> {
        self.ensure_project(project).await?;
        Ok(self
            .caster
            .inspect_reference(project, voice_id, reference_id)
            .await?)
    }

    pub async fn check_compatibility(
        &self,
        project: &ProjectId,
        voice_id: String,
        engine_id: String,
        emotion_id: String,
    ) -> Result<VoiceCompatibility, ApplicationError> {
        self.ensure_project(project).await?;
        Ok(self
            .caster
            .check_compatibility(project, voice_id, engine_id, emotion_id)
            .await?)
    }

    pub async fn current_cast(
        &self,
        project: &ProjectId,
    ) -> Result<Option<CastResult>, ApplicationError> {
        let lock = self.locks.get_lock(project)?;
        let _guard = lock.lock().await;
        self.ensure_project(project).await?;
        let current = self.caster.current_cast(project).await?;
        if let Some(ref result) = current {
            self.register_revision(project, result).await?;
        }
        Ok(current)
    }

    pub async fn assign_roles(
        &self,
        project: &ProjectId,
        cast: LogicalCast,
        selections: BTreeMap<String, ManualSelection>,
        expected_revision: Option<String>,
        request_id: String,
    ) -> Result<CastResult, ApplicationError> {
        let lock = self.locks.get_lock(project)?;
        let _guard = lock.lock().await;
        self.ensure_project(project).await?;
        let result = self
            .caster
            .assign_roles(project, cast, selections, expected_revision, request_id)
            .await?;
        self.register_revision(project, &result).await?;
        Ok(result)
    }

    pub async fn recover_cast(&self, project: &ProjectId) -> Result<String, ApplicationError> {
        let lock = self.locks.get_lock(project)?;
        let _guard = lock.lock().await;
        self.ensure_project(project).await?;
        let status = self.caster.recover_cast(project).await?;
        if let Some(result) = self.caster.current_cast(project).await? {
            self.register_revision(project, &result).await?;
        }
        Ok(status)
    }
}
