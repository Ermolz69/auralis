use async_trait::async_trait;

use crate::error::PortError;
use crate::project_update::ProjectUpdate;
use domain::job::{Job, JobId};
use domain::project::{Project, ProjectId};

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: Project) -> Result<Project, PortError>;
    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError>;
    async fn update(
        &self,
        id: &ProjectId,
        expected_revision: u64,
        update: ProjectUpdate,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Project, PortError>;
    async fn list(&self) -> Result<Vec<Project>, PortError>;
    async fn delete(&self, id: &ProjectId) -> Result<(), PortError>;
}

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn create(&self, job: Job) -> Result<Job, PortError>;
    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError>;
    async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError>;
    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError>;
    async fn list_active(&self) -> Result<Vec<Job>, PortError>;
    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError>;
}

use domain::outbox::{OutboxMessage, OutboxMessageId};

pub struct FetchPendingResult {
    pub messages: Vec<OutboxMessage>,
    pub corrupted_isolated: usize,
    pub isolation_errors: usize,
}

pub struct OutboxClaim {
    pub message: OutboxMessage,
    pub token: String,
}

pub struct ClaimReadyResult {
    pub claims: Vec<OutboxClaim>,
    pub corrupted_isolated: usize,
    pub isolation_errors: usize,
}

#[derive(Default)]
pub struct OutboxProtectedResources {
    pub staging_keys: Vec<String>,
    pub workspace_keys: Vec<domain::outbox::WorkspaceKey>,
}

pub struct OutboxPruneReport {
    pub done_deleted: usize,
    pub dead_deleted: usize,
}

#[async_trait]
pub trait OutboxRepository: Send + Sync {
    async fn protected_resources(&self) -> Result<OutboxProtectedResources, PortError> {
        Err(PortError::Unsupported {
            message: "Outbox storage must report protected staging and workspace resources".into(),
        })
    }
    async fn claim_ready(
        &self,
        _limit: usize,
        _lease_timeout: std::time::Duration,
    ) -> Result<ClaimReadyResult, PortError> {
        Err(PortError::Unsupported {
            message: "Outbox storage must implement token-fenced claims".into(),
        })
    }
    async fn renew_claim(&self, _id: &OutboxMessageId, _token: &str) -> Result<bool, PortError> {
        Err(PortError::Unsupported {
            message: "Outbox storage must implement token-fenced renewal".into(),
        })
    }
    async fn claim_is_current(
        &self,
        _id: &OutboxMessageId,
        _token: &str,
    ) -> Result<bool, PortError> {
        Err(PortError::Unsupported {
            message: "Outbox storage must implement token-fenced ownership checks".into(),
        })
    }
    async fn mark_done_claim(
        &self,
        _id: &OutboxMessageId,
        _token: &str,
    ) -> Result<bool, PortError> {
        Err(PortError::Unsupported {
            message: "Outbox storage must implement token-fenced acknowledgement".into(),
        })
    }
    async fn mark_failed_claim(
        &self,
        _id: &OutboxMessageId,
        _token: &str,
        _error: &str,
    ) -> Result<bool, PortError> {
        Err(PortError::Unsupported {
            message: "Outbox storage must implement token-fenced failure writes".into(),
        })
    }
    async fn fetch_pending(&self, limit: usize) -> Result<FetchPendingResult, PortError>;
    async fn mark_processing(
        &self,
        id: &OutboxMessageId,
        locked_by: &str,
    ) -> Result<bool, PortError>;
    async fn mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError>;
    async fn mark_failed(&self, id: &OutboxMessageId, error: &str) -> Result<(), PortError>;
    async fn mark_dead_raw(&self, id_raw: &str, reason: &str) -> Result<(), PortError>;

    async fn prune_terminal_rows(
        &self,
        done_before: chrono::DateTime<chrono::Utc>,
        dead_before: chrono::DateTime<chrono::Utc>,
        per_status_batch_limit: u32,
    ) -> Result<OutboxPruneReport, PortError>;
}
use std::sync::Arc;

#[async_trait]
impl<T> ProjectRepository for Arc<T>
where
    T: ProjectRepository + ?Sized,
{
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        (**self).create(project).await
    }

    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        (**self).get(id).await
    }

    async fn update(
        &self,
        id: &ProjectId,
        expected_revision: u64,
        update: ProjectUpdate,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Project, PortError> {
        (**self)
            .update(id, expected_revision, update, updated_at)
            .await
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        (**self).list().await
    }

    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        (**self).delete(id).await
    }
}

#[async_trait]
impl<T> OutboxRepository for Arc<T>
where
    T: OutboxRepository + ?Sized,
{
    async fn protected_resources(&self) -> Result<OutboxProtectedResources, PortError> {
        (**self).protected_resources().await
    }
    async fn claim_ready(
        &self,
        limit: usize,
        lease_timeout: std::time::Duration,
    ) -> Result<ClaimReadyResult, PortError> {
        (**self).claim_ready(limit, lease_timeout).await
    }
    async fn renew_claim(&self, id: &OutboxMessageId, token: &str) -> Result<bool, PortError> {
        (**self).renew_claim(id, token).await
    }
    async fn claim_is_current(&self, id: &OutboxMessageId, token: &str) -> Result<bool, PortError> {
        (**self).claim_is_current(id, token).await
    }
    async fn mark_done_claim(&self, id: &OutboxMessageId, token: &str) -> Result<bool, PortError> {
        (**self).mark_done_claim(id, token).await
    }
    async fn mark_failed_claim(
        &self,
        id: &OutboxMessageId,
        token: &str,
        error: &str,
    ) -> Result<bool, PortError> {
        (**self).mark_failed_claim(id, token, error).await
    }
    async fn fetch_pending(&self, limit: usize) -> Result<FetchPendingResult, PortError> {
        (**self).fetch_pending(limit).await
    }

    async fn mark_processing(
        &self,
        id: &OutboxMessageId,
        locked_by: &str,
    ) -> Result<bool, PortError> {
        (**self).mark_processing(id, locked_by).await
    }

    async fn mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError> {
        (**self).mark_done(id).await
    }

    async fn mark_failed(&self, id: &OutboxMessageId, error: &str) -> Result<(), PortError> {
        (**self).mark_failed(id, error).await
    }

    async fn mark_dead_raw(&self, id_raw: &str, reason: &str) -> Result<(), PortError> {
        (**self).mark_dead_raw(id_raw, reason).await
    }

    async fn prune_terminal_rows(
        &self,
        done_before: chrono::DateTime<chrono::Utc>,
        dead_before: chrono::DateTime<chrono::Utc>,
        per_status_batch_limit: u32,
    ) -> Result<OutboxPruneReport, PortError> {
        (**self)
            .prune_terminal_rows(done_before, dead_before, per_status_batch_limit)
            .await
    }
}
