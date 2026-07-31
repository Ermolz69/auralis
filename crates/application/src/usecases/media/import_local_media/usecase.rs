use std::path::PathBuf;
use std::sync::Arc;

use domain::project::{Project, ProjectId};
use ports::media::MediaProbePort;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;

use super::cleanup::cleanup_after_stage;
use crate::error::ApplicationError;
use crate::usecases::media::probe_local::{ProbeLocalMediaRequest, ProbeLocalMediaUseCase};

#[derive(Debug)]
pub struct ImportLocalMediaRequest {
    pub project_id: ProjectId,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ImportLocalMediaResponse {
    pub project: Project,
}

pub struct ImportLocalMediaUseCase<
    R: ProjectRepository + Clone + 'static,
    P: MediaProbePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    project_repo: R,
    media_probe: P,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    artifact_store: S,
    locks: Arc<crate::usecases::project::lifecycle::ProjectLifecycleLocks>,
}

impl<
    R: ProjectRepository + Clone + 'static,
    P: MediaProbePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> ImportLocalMediaUseCase<R, P, S>
{
    pub fn new(
        project_repo: R,
        probe: P,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        artifact_store: S,
        locks: Arc<crate::usecases::project::lifecycle::ProjectLifecycleLocks>,
    ) -> Self {
        Self {
            project_repo,
            media_probe: probe,
            storage_uow,
            artifact_store,
            locks,
        }
    }

    pub async fn execute(
        &self,
        request: ImportLocalMediaRequest,
    ) -> Result<ImportLocalMediaResponse, ApplicationError> {
        // 1. Initial load & validation (outside lock)
        let project_opt = self.project_repo.get(&request.project_id).await?;
        let project = project_opt
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;
        if project.status() != &domain::project::ProjectStatus::Draft {
            return Err(ApplicationError::InvalidOperation {
                message: format!(
                    "Project must be in Draft status, current: {:?}",
                    project.status()
                ),
            });
        }
        let original_updated_at = project.updated_at();

        // 2. Read-only long operations (probe & stage, outside lock)
        let probe_use_case = ProbeLocalMediaUseCase::new(self.media_probe.clone());
        let probe_req = ProbeLocalMediaRequest {
            path: request.path.clone(),
        };
        let probe_res = probe_use_case.execute(probe_req).await?;

        let original_filename = request
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string());

        let staged_artifact = self
            .artifact_store
            .import_external_file(
                &request.project_id,
                domain::media::ArtifactKind::SourceVideo,
                &request.path,
                original_filename.as_deref(),
            )
            .await?;

        let staging_key = staged_artifact.staging_key.clone();

        // 3. DB mutation (inside lifecycle lock async block)
        let commit_result: Result<Project, ApplicationError> = async {
            let lock_arc = self.locks.get_lock(&request.project_id)?;
            let _guard = lock_arc.lock().await;

            // Revalidate existence, status, and updated_at
            let reloaded_opt = self.project_repo.get(&request.project_id).await?;
            let mut reloaded = reloaded_opt
                .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

            if reloaded.status() != &domain::project::ProjectStatus::Draft {
                return Err(ApplicationError::InvalidOperation {
                    message: "Project status changed concurrently".to_string(),
                });
            }

            if reloaded.updated_at() != original_updated_at {
                return Err(ApplicationError::InvalidOperation {
                    message: "Project was modified concurrently".to_string(),
                });
            }

            // Domain transitions
            let source = domain::media::MediaSource::ManagedLocalFile {
                artifact_id: staged_artifact.artifact.id.clone(),
                original_filename: original_filename.unwrap_or_else(|| "video.mp4".to_string()),
            };
            reloaded.import_source(source, Some(probe_res.metadata.clone()))?;
            reloaded.mark_ready_for_processing()?;

            // Commit UoW
            let cmd = ports::transaction::CommitManagedSourceImport {
                project: reloaded.clone(),
                artifact: staged_artifact.artifact.clone(),
                staging_key: staged_artifact.staging_key.clone(),
                final_key: staged_artifact.final_key.clone(),
                original_updated_at,
            };

            self.storage_uow.commit_managed_source_import(cmd).await?;

            Ok(reloaded)
        }
        .await;

        // 4. Post-stage cleanup handling after lock release
        match commit_result {
            Ok(final_project) => Ok(ImportLocalMediaResponse {
                project: final_project,
            }),
            Err(primary) => {
                Err(cleanup_after_stage(primary, &staging_key, &self.artifact_store).await)
            }
        }
    }
}
