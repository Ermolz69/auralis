#![allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
// Module is defined externally as tests.rs
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use domain::job::{Job, JobKind};
use domain::project::{Project, ProjectStatus};
use domain::system::recovery::RecoveryIssueType;
use ports::error::PortError;
use ports::recovery::{
    FailInterruptedPairCommand, FailLegacyPairFallbackCommand, FailLegacyProjectWithoutJobCommand,
    FailOrphanJobCommand, FailProjectWithMissingLinkedJobCommand, ReconcileTerminalPairCommand,
    RecoveryApplyResult, RecoverySnapshot, RecoveryStorage,
};

use crate::usecases::system::recover_interrupted::usecase::RecoverInterruptedStateUseCase;

struct MockRecoveryRepo {
    pub snapshot: Mutex<Option<RecoverySnapshot>>,
    pub apply_calls: Mutex<usize>,
    pub results: Mutex<Vec<Result<RecoveryApplyResult, PortError>>>,
}

impl MockRecoveryRepo {
    pub fn new(snapshot: RecoverySnapshot) -> Self {
        Self {
            snapshot: Mutex::new(Some(snapshot)),
            apply_calls: Mutex::new(0),
            results: Mutex::new(Vec::new()),
        }
    }

    pub fn with_results(
        snapshot: RecoverySnapshot,
        results: Vec<Result<RecoveryApplyResult, PortError>>,
    ) -> Self {
        Self {
            snapshot: Mutex::new(Some(snapshot)),
            apply_calls: Mutex::new(0),
            results: Mutex::new(results),
        }
    }

    async fn take_result(&self) -> Result<RecoveryApplyResult, PortError> {
        *self.apply_calls.lock().await += 1;
        let mut res = self.results.lock().await;
        if !res.is_empty() {
            res.remove(0)
        } else {
            Ok(RecoveryApplyResult::Applied)
        }
    }
}

#[async_trait]
impl RecoveryStorage for MockRecoveryRepo {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError> {
        Ok(self.snapshot.lock().await.take().unwrap())
    }

    async fn commit_failed_interrupted_pair(
        &self,
        _cmd: FailInterruptedPairCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        self.take_result().await
    }

    async fn commit_reconciled_terminal_pair(
        &self,
        _cmd: ReconcileTerminalPairCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        self.take_result().await
    }

    async fn commit_legacy_pair_fallback(
        &self,
        _cmd: FailLegacyPairFallbackCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        self.take_result().await
    }

    async fn commit_failed_project_with_missing_linked_job(
        &self,
        _cmd: FailProjectWithMissingLinkedJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        self.take_result().await
    }

    async fn commit_failed_legacy_project_without_job(
        &self,
        _cmd: FailLegacyProjectWithoutJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        self.take_result().await
    }

    async fn commit_failed_orphan_job(
        &self,
        _cmd: FailOrphanJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        self.take_result().await
    }
}

fn make_project_processing(id: &str, active_job_id: Option<domain::job::JobId>) -> Project {
    let p = Project::new(id.to_string());
    let mut snap = p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = active_job_id;
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    Project::from_snapshot(snap).unwrap()
}

#[tokio::test]
async fn test_duplicate_active_job_id_blocks_projects_and_jobs() {
    let mut job = Job::new(
        domain::project::ProjectId::new(),
        "test_proj".into(),
        JobKind::Dubbing,
    );
    let _ = job.start();

    let p1 = make_project_processing("Proj1", Some(job.id().clone()));
    let p2 = make_project_processing("Proj2", Some(job.id().clone()));

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1, p2],
        linked_jobs: vec![job],
        active_jobs: vec![],
    };

    let repo = Arc::new(MockRecoveryRepo::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    // Planner should block everything and emit unresolved violation
    assert_eq!(*repo.apply_calls.lock().await, 0);
    assert_eq!(result.resolved_violations.len(), 0);
    assert_eq!(result.unresolved_violations.len(), 1);
    assert_eq!(
        result.unresolved_violations[0].issue_type,
        RecoveryIssueType::MultipleActiveJobs
    );
}

#[tokio::test]
async fn test_legacy_ambiguity_blocks_project() {
    let p1 = make_project_processing("Proj1", None);

    let mut job1 = Job::new(p1.id().clone(), "j1".into(), JobKind::Dubbing);
    let _ = job1.start();

    let mut job2 = Job::new(p1.id().clone(), "j2".into(), JobKind::Dubbing);
    let _ = job2.start();

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1],
        linked_jobs: vec![],
        active_jobs: vec![job1, job2],
    };

    let repo = Arc::new(MockRecoveryRepo::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    // Planner should block the project and both jobs
    assert_eq!(*repo.apply_calls.lock().await, 0);
    assert_eq!(result.unresolved_violations.len(), 1);
    assert_eq!(
        result.unresolved_violations[0].issue_type,
        RecoveryIssueType::AmbiguousLegacyJobs
    );
}

#[tokio::test]
async fn test_missing_active_job_resolves_and_fails_project() {
    let p1 = make_project_processing("Proj1", Some(domain::job::JobId::new()));

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1],
        linked_jobs: vec![],
        active_jobs: vec![],
    };

    let repo = Arc::new(MockRecoveryRepo::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    assert_eq!(*repo.apply_calls.lock().await, 1);
    assert_eq!(result.resolved_violations.len(), 1);
    assert_eq!(
        result.resolved_violations[0].issue_type,
        RecoveryIssueType::MissingActiveJob
    );
}

#[tokio::test]
async fn test_orphan_job_creates_resolved_violation() {
    let mut job = Job::new(
        domain::project::ProjectId::new(),
        "test_proj".into(),
        JobKind::Dubbing,
    );
    let _ = job.start();

    let snapshot = RecoverySnapshot {
        processing_projects: vec![],
        linked_jobs: vec![],
        active_jobs: vec![job],
    };

    let repo = Arc::new(MockRecoveryRepo::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    assert_eq!(*repo.apply_calls.lock().await, 1);
    assert_eq!(result.resolved_violations.len(), 1);
    assert_eq!(
        result.resolved_violations[0].issue_type,
        RecoveryIssueType::OrphanActiveJob
    );
}

#[tokio::test]
async fn test_legacy_pair_creates_resolved_violation() {
    let p1 = make_project_processing("Proj1", None);
    let mut job = Job::new(p1.id().clone(), "j1".into(), JobKind::Dubbing);
    let _ = job.start();

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1],
        linked_jobs: vec![],
        active_jobs: vec![job],
    };

    let repo = Arc::new(MockRecoveryRepo::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    assert_eq!(*repo.apply_calls.lock().await, 1);
    assert_eq!(result.resolved_violations.len(), 1);
    assert_eq!(
        result.resolved_violations[0].issue_type,
        RecoveryIssueType::MissingActiveJob
    );
}

#[tokio::test]
async fn test_recovery_action_fails_but_next_is_applied() {
    let p1 = make_project_processing("Proj1", Some(domain::job::JobId::new()));
    let p2 = make_project_processing("Proj2", Some(domain::job::JobId::new()));

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1, p2],
        linked_jobs: vec![],
        active_jobs: vec![],
    };

    let repo = Arc::new(MockRecoveryRepo::with_results(
        snapshot,
        vec![
            Err(PortError::Unexpected {
                message: "DB error 1".into(),
            }),
            Ok(RecoveryApplyResult::Applied),
        ],
    ));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    assert_eq!(*repo.apply_calls.lock().await, 2);
    assert_eq!(result.actions_applied, 1);
    assert_eq!(result.persistence_failures.len(), 1);
}

#[tokio::test]
async fn test_multiple_persistence_failures_aggregate() {
    let p1 = make_project_processing("Proj1", Some(domain::job::JobId::new()));
    let p2 = make_project_processing("Proj2", Some(domain::job::JobId::new()));

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1, p2],
        linked_jobs: vec![],
        active_jobs: vec![],
    };

    let repo = Arc::new(MockRecoveryRepo::with_results(
        snapshot,
        vec![
            Err(PortError::Unexpected {
                message: "DB error 1".into(),
            }),
            Err(PortError::Unexpected {
                message: "DB error 2".into(),
            }),
        ],
    ));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    assert_eq!(*repo.apply_calls.lock().await, 2);
    assert_eq!(result.actions_applied, 0);
    assert_eq!(result.persistence_failures.len(), 2);
}

#[tokio::test]
async fn test_failed_action_is_not_resolved() {
    let p1 = make_project_processing("Proj1", Some(domain::job::JobId::new()));

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1],
        linked_jobs: vec![],
        active_jobs: vec![],
    };

    let repo = Arc::new(MockRecoveryRepo::with_results(
        snapshot,
        vec![Err(PortError::Unexpected {
            message: "DB error 1".into(),
        })],
    ));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    // The action fails, so the violation should NOT be added to resolved_violations
    assert_eq!(result.resolved_violations.len(), 0);
    assert_eq!(result.persistence_failures.len(), 1);
}

#[tokio::test]
async fn test_already_applied_becomes_resolved_but_not_incremented() {
    let p1 = make_project_processing("Proj1", Some(domain::job::JobId::new()));

    let snapshot = RecoverySnapshot {
        processing_projects: vec![p1],
        linked_jobs: vec![],
        active_jobs: vec![],
    };

    let repo = Arc::new(MockRecoveryRepo::with_results(
        snapshot,
        vec![Ok(RecoveryApplyResult::AlreadyApplied)],
    ));
    let usecase = RecoverInterruptedStateUseCase::new(repo.clone());
    let result = usecase.execute().await.unwrap();

    // The action was already applied, so we still consider the violation resolved
    // but actions_applied must be 0
    assert_eq!(result.resolved_violations.len(), 1);
    assert_eq!(result.actions_applied, 0);
    assert_eq!(result.persistence_failures.len(), 0);
}
