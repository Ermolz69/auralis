use super::super::lifecycle::ProjectLifecycleLocks;
use crate::error::ApplicationError;
use domain::{
    media::MediaSource,
    project::{Project, ProjectId, ProjectStatus},
};
use ports::{
    error::PortError,
    repository::ProjectRepository,
    source::VideoSourcePort,
    storage::ArtifactStore,
    transaction::CommitYoutubeImport,
    workspace::TempWorkspacePort,
    youtube_import::{YoutubeImportJournal, YoutubeImportSession, YoutubeImportState},
};
use std::sync::Arc;

pub struct CreateProjectFromYoutubeRequest {
    pub url: String,
    pub project_id: Option<ProjectId>,
}

pub struct CreateProjectFromYoutubeResponse {
    pub project: Project,
}

pub struct CreateProjectFromYoutubeUseCase<R, V, S> {
    pub(super) project_repo: R,
    pub(super) video_source: V,
    pub(super) artifact_store: S,
    pub(super) journal: Arc<dyn YoutubeImportJournal>,
    pub(super) workspace_port: Arc<dyn TempWorkspacePort>,
    pub(super) locks: Arc<ProjectLifecycleLocks>,
}

impl<R: ProjectRepository, V: VideoSourcePort + Clone, S: ArtifactStore + Clone>
    CreateProjectFromYoutubeUseCase<R, V, S>
{
    pub fn new(
        project_repo: R,
        video_source: V,
        artifact_store: S,
        journal: Arc<dyn YoutubeImportJournal>,
        workspace_port: Arc<dyn TempWorkspacePort>,
        locks: Arc<ProjectLifecycleLocks>,
    ) -> Self {
        Self {
            project_repo,
            video_source,
            artifact_store,
            journal,
            workspace_port,
            locks,
        }
    }

    pub async fn list_pending(&self) -> Result<Vec<YoutubeImportSession>, ApplicationError> {
        Ok(self.journal.list().await?)
    }

    pub async fn discard(&self, id: &ProjectId) -> Result<(), ApplicationError> {
        let _lease = self.workspace_port.acquire_import_lease(id).await?;
        let session = self
            .journal
            .list()
            .await?
            .into_iter()
            .find(|session| &session.project.id == id)
            .ok_or_else(|| ApplicationError::ProjectNotFound(id.clone()))?;
        self.journal.discard(id, session.revision).await?;
        Ok(())
    }

    pub async fn execute(
        &self,
        request: CreateProjectFromYoutubeRequest,
    ) -> Result<CreateProjectFromYoutubeResponse, ApplicationError> {
        let url = request.url.trim();
        let key = match &request.project_id {
            Some(id) => format!("project:{id}"),
            None => format!("url:{url}"),
        };
        let id = if let Some(session) = self.journal.find(&key).await? {
            if session.url() != url {
                return Err(conflict(
                    "Another source import is pending; discard it first",
                ));
            }
            session.project.id
        } else {
            self.prepare(url, request.project_id).await?
        };
        self.resume(&id).await
    }

    pub async fn resume(
        &self,
        project_id: &ProjectId,
    ) -> Result<CreateProjectFromYoutubeResponse, ApplicationError> {
        let initial = self
            .journal
            .list()
            .await?
            .into_iter()
            .find(|session| &session.project.id == project_id)
            .ok_or_else(|| conflict("Import is already completed or discarded"))?;
        let request_key = initial.request_key();
        let _lease = self
            .workspace_port
            .acquire_import_lease(&initial.project.id)
            .await?;
        let mut session = self
            .journal
            .find(&request_key)
            .await?
            .filter(|current| current.project.id == initial.project.id)
            .ok_or_else(|| conflict("Import is already completed or discarded"))?;
        let result = async {
            self.refresh_baseline(&mut session).await?;
            self.resume_inner(&mut session).await
        }
        .await;
        if result.is_err()
            && self
                .journal
                .find(&request_key)
                .await?
                .is_some_and(|current| {
                    current.project.id == session.project.id && current.revision == session.revision
                })
        {
            session.state = YoutubeImportState::Failed;
            self.journal.checkpoint(&mut session).await?;
        }
        result
    }

    async fn refresh_baseline(
        &self,
        session: &mut YoutubeImportSession,
    ) -> Result<(), ApplicationError> {
        if session.original_updated_at.is_none() {
            return Ok(());
        }
        let mut current = self
            .project_repo
            .get(&session.project.id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(session.project.id.clone()))?;
        ensure_draft(&current)?;
        if current.revision() != session.project.revision {
            session.original_updated_at = Some(current.updated_at());
            current.import_source(
                MediaSource::YoutubeUrl {
                    url: session.url().into(),
                },
                session.project.metadata.clone(),
            )?;
            current.mark_ready_for_processing()?;
            session.project = current.to_snapshot();
            self.journal.checkpoint(session).await?;
        }
        Ok(())
    }

    async fn revalidate(&self, session: &YoutubeImportSession) -> Result<(), ApplicationError> {
        if session.original_updated_at.is_some() {
            let current = self
                .project_repo
                .get(&session.project.id)
                .await?
                .ok_or_else(|| ApplicationError::ProjectNotFound(session.project.id.clone()))?;
            ensure_draft(&current)?;
            if current.revision() != session.project.revision {
                return Err(conflict(
                    "Project changed during download; resume again to use its latest revision",
                ));
            }
        }
        Ok(())
    }

    async fn resume_inner(
        &self,
        session: &mut YoutubeImportSession,
    ) -> Result<CreateProjectFromYoutubeResponse, ApplicationError> {
        self.revalidate(session).await?;
        self.stage(session).await?;
        let lock = self.locks.get_lock(&session.project.id)?;
        let _guard = lock.lock().await;
        self.revalidate(session).await?;
        let project = Project::from_snapshot(session.project.clone())?;
        let write = session
            .write
            .clone()
            .ok_or_else(|| conflict("Missing import checkpoint"))?;
        self.journal
            .commit(
                session,
                CommitYoutubeImport {
                    project: project.clone(),
                    write,
                    original_updated_at: session.original_updated_at,
                },
            )
            .await?;
        let mut committed = project;
        if session.original_updated_at.is_some() {
            committed.advance_revision()?;
        }
        Ok(CreateProjectFromYoutubeResponse { project: committed })
    }
}

pub(super) fn ensure_draft(project: &Project) -> Result<(), ApplicationError> {
    if project.status() != &ProjectStatus::Draft || project.active_job_id().is_some() {
        return Err(ApplicationError::InvalidOperation {
            message: "YouTube import requires a Draft project without an active job".into(),
        });
    }
    Ok(())
}

pub(super) fn conflict(message: &str) -> ApplicationError {
    PortError::Conflict {
        resource: "YouTube import".into(),
        message: message.into(),
    }
    .into()
}
