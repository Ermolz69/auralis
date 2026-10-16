#![allow(clippy::unwrap_used, clippy::expect_used)]

use adapters_storage::memory::{
    InMemoryArtifactIndex, InMemoryDatabase, InMemoryProjectRepository,
};
use application::usecases::caster::CasterUseCases;
use application::usecases::project::lifecycle::ProjectLifecycleLocks;
use async_trait::async_trait;
use domain::media::{ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::{Project, ProjectId};
use ports::artifact_index::ArtifactIndex;
use ports::caster::{
    CastResult, CastRevision, CastRole, CasterPort, LogicalCast, ManualSelection, ReferenceDetails,
    VoiceCatalogPage, VoiceCompatibility, VoiceDetails,
};
use ports::error::PortError;
use ports::repository::ProjectRepository;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

struct FixtureCaster {
    result: CastResult,
}

#[async_trait]
impl CasterPort for FixtureCaster {
    async fn catalog_page(
        &self,
        _: &ProjectId,
        _: Option<String>,
        _: u32,
    ) -> Result<VoiceCatalogPage, PortError> {
        unreachable!()
    }
    async fn inspect_voice(
        &self,
        _: &ProjectId,
        _: String,
        _: Option<String>,
        _: u32,
    ) -> Result<VoiceDetails, PortError> {
        unreachable!()
    }
    async fn inspect_reference(
        &self,
        _: &ProjectId,
        _: String,
        _: String,
    ) -> Result<ReferenceDetails, PortError> {
        unreachable!()
    }
    async fn check_compatibility(
        &self,
        _: &ProjectId,
        _: String,
        _: String,
        _: String,
    ) -> Result<VoiceCompatibility, PortError> {
        unreachable!()
    }
    async fn current_cast(&self, _: &ProjectId) -> Result<Option<CastResult>, PortError> {
        Ok(Some(self.result.clone()))
    }
    async fn assign_roles(
        &self,
        _: &ProjectId,
        _: LogicalCast,
        _: BTreeMap<String, ManualSelection>,
        _: Option<String>,
        _: String,
    ) -> Result<CastResult, PortError> {
        Ok(self.result.clone())
    }
    async fn recover_cast(&self, _: &ProjectId) -> Result<String, PortError> {
        Ok("completed".into())
    }
}

#[tokio::test]
async fn caster_revision_is_indexed_once_and_reconciled_on_read() {
    let database = Arc::new(Mutex::new(InMemoryDatabase::default()));
    let projects = Arc::new(InMemoryProjectRepository::new(database));
    let project = projects
        .create(Project::new("Caster fixture".into()).unwrap())
        .await
        .unwrap();
    let artifacts = Arc::new(InMemoryArtifactIndex::new());
    let revision_sha256 = "a".repeat(64);
    let result = CastResult {
        revision_sha256: revision_sha256.clone(),
        revision: CastRevision {
            version: 1,
            parent_revision: None,
            request_id: "request-1".into(),
            request_sha256: "b".repeat(64),
            cast: LogicalCast {
                version: 2,
                roles: BTreeMap::from([(
                    "narrator".into(),
                    CastRole {
                        display_name: "Narrator".into(),
                        voice_id: None,
                    },
                )]),
            },
            selections: BTreeMap::new(),
        },
    };
    let usecases = CasterUseCases::new(
        projects,
        artifacts.clone(),
        Arc::new(FixtureCaster { result }),
        Arc::new(ProjectLifecycleLocks::new()),
    );
    let project_id = project.id();
    let current = usecases.current_cast(project_id).await.unwrap().unwrap();
    assert_eq!(current.revision_sha256, revision_sha256);
    let indexed = artifacts
        .list_by_project_and_kind(project_id, ArtifactKind::ManualCast)
        .await
        .unwrap();
    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].state, ArtifactState::Ready);
    assert_eq!(
        indexed[0].location,
        ArtifactLocation::StorageKey(format!(
            "{project_id}/casting/revisions/{revision_sha256}.json"
        ))
    );
    usecases
        .assign_roles(
            project_id,
            current.revision.cast,
            BTreeMap::new(),
            None,
            "request-1".into(),
        )
        .await
        .unwrap();
    usecases.recover_cast(project_id).await.unwrap();
    assert_eq!(
        artifacts
            .list_by_project_and_kind(project_id, ArtifactKind::ManualCast)
            .await
            .unwrap()
            .len(),
        1
    );
}
