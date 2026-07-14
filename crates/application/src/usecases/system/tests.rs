use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::recover_interrupted::RecoverInterruptedStateUseCase;
use domain::job::{Job, JobError, JobId, JobStatus};
use domain::project::{Project, ProjectId, ProjectStatus};
use domain::system::recovery::RecoveryIssueType;
use ports::error::PortError;
use ports::recovery::{RecoverySnapshot, RecoveryStorage};

struct MockRecoveryStorage {
    snapshot: RecoverySnapshot,
    interrupted_pairs: Mutex<Vec<(Project, Job)>>,
    reconciled_projects: Mutex<Vec<Project>>,
    failed_projects_no_job: Mutex<Vec<Project>>,
    orphan_jobs: Mutex<Vec<Job>>,
}

impl MockRecoveryStorage {
    fn new(snapshot: RecoverySnapshot) -> Self {
        Self {
            snapshot,
            interrupted_pairs: Mutex::new(Vec::new()),
            reconciled_projects: Mutex::new(Vec::new()),
            failed_projects_no_job: Mutex::new(Vec::new()),
            orphan_jobs: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl RecoveryStorage for MockRecoveryStorage {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError> {
        Ok(RecoverySnapshot {
            processing_projects: self.snapshot.processing_projects.clone(),
            linked_jobs: self.snapshot.linked_jobs.clone(),
            active_jobs: self.snapshot.active_jobs.clone(),
        })
    }

    async fn commit_interrupted_pair(&self, project: Project, job: Job) -> Result<(), PortError> {
        self.interrupted_pairs.lock().await.push((project, job));
        Ok(())
    }

    async fn commit_reconciled_project(&self, project: Project) -> Result<(), PortError> {
        self.reconciled_projects.lock().await.push(project);
        Ok(())
    }

    async fn commit_failed_project_no_job(&self, project: Project) -> Result<(), PortError> {
        self.failed_projects_no_job.lock().await.push(project);
        Ok(())
    }

    async fn commit_orphan_job(&self, job: Job) -> Result<(), PortError> {
        self.orphan_jobs.lock().await.push(job);
        Ok(())
    }
}

use domain::job::{JobKind, JobProgress, JobSnapshot};

fn create_project(status: ProjectStatus, active_job_id: Option<JobId>) -> Project {
    let mut p = Project::new("Test".into());
    // Use transitions to reach desired state
    match status {
        ProjectStatus::Draft => {}
        ProjectStatus::SourceImported => {
            p.import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "test".into(),
                },
                None,
            )
            .unwrap();
        }
        ProjectStatus::ReadyForProcessing => {
            p.import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "test".into(),
                },
                None,
            )
            .unwrap();
            p.mark_ready_for_processing().unwrap();
        }
        ProjectStatus::Processing => {
            p.import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "test".into(),
                },
                None,
            )
            .unwrap();
            p.mark_ready_for_processing().unwrap();
            p.start_processing(active_job_id.unwrap_or_default())
                .unwrap();
        }
        ProjectStatus::Completed => {
            p.import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "test".into(),
                },
                None,
            )
            .unwrap();
            p.mark_ready_for_processing().unwrap();
            let jid = JobId::new();
            p.start_processing(jid.clone()).unwrap();
            p.apply_terminal_transition(&jid, domain::job::TerminalOutcome::Completed)
                .unwrap();
        }
        ProjectStatus::Failed => {
            p.force_failed_due_to_recovery();
        }
        ProjectStatus::Cancelled => {
            p.import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "test".into(),
                },
                None,
            )
            .unwrap();
            p.mark_ready_for_processing().unwrap();
            let jid = JobId::new();
            p.start_processing(jid.clone()).unwrap();
            p.apply_terminal_transition(&jid, domain::job::TerminalOutcome::Cancelled)
                .unwrap();
        }
    }
    p
}

fn create_job(project_id: ProjectId, status: JobStatus, id: Option<JobId>) -> Job {
    let snapshot = JobSnapshot {
        id: id.unwrap_or_default(),
        project_id,
        title: "Test Job".into(),
        kind: JobKind::Dubbing,
        status,
        stage: None,
        progress: JobProgress::initializing(),
        error: None,
        created_at: domain::chrono::Utc::now(),
        updated_at: domain::chrono::Utc::now(),
        started_at: None,
        finished_at: None,
    };
    Job::from_snapshot(snapshot)
}

#[tokio::test]
async fn test_processing_pending_both_failed() {
    let job_id = JobId::new();
    let project = create_project(ProjectStatus::Processing, Some(job_id.clone()));
    let job = create_job(
        project.id().clone(),
        JobStatus::Pending,
        Some(job_id.clone()),
    );

    let snapshot = RecoverySnapshot {
        processing_projects: vec![project.clone()],
        linked_jobs: vec![job.clone()],
        active_jobs: vec![],
    };

    let mock = Arc::new(MockRecoveryStorage::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(mock.clone());

    let report = usecase.execute().await.unwrap();
    assert_eq!(report.recovered_pairs, 1);
    assert_eq!(report.reconciled_terminal_projects, 0);
    assert_eq!(report.recovered_orphan_jobs, 0);
    assert!(report.warnings.is_empty());
    assert!(!report.has_fatal_issues());

    let pairs = mock.interrupted_pairs.lock().await;
    assert_eq!(pairs.len(), 1);
    assert_eq!(*pairs[0].0.status(), ProjectStatus::Failed);
    assert_eq!(*pairs[0].1.status(), JobStatus::Failed);
    assert_eq!(pairs[0].1.error().as_ref().unwrap().code, "APP_RESTART");
}

#[tokio::test]
async fn test_processing_completed_reconciled() {
    let job_id = JobId::new();
    let project = create_project(ProjectStatus::Processing, Some(job_id.clone()));
    let job = create_job(
        project.id().clone(),
        JobStatus::Completed,
        Some(job_id.clone()),
    );

    let snapshot = RecoverySnapshot {
        processing_projects: vec![project.clone()],
        linked_jobs: vec![job.clone()],
        active_jobs: vec![],
    };

    let mock = Arc::new(MockRecoveryStorage::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(mock.clone());

    let report = usecase.execute().await.unwrap();
    assert_eq!(report.recovered_pairs, 0);
    assert_eq!(report.reconciled_terminal_projects, 1);

    let reconciled = mock.reconciled_projects.lock().await;
    assert_eq!(reconciled.len(), 1);
    assert_eq!(*reconciled[0].status(), ProjectStatus::Completed);
}

#[tokio::test]
async fn test_processing_failed_reconciled() {
    let job_id = JobId::new();
    let project = create_project(ProjectStatus::Processing, Some(job_id.clone()));
    let mut job = create_job(
        project.id().clone(),
        JobStatus::Failed,
        Some(job_id.clone()),
    );
    job.mark_failed(JobError::new("PREV", "PREV_ERR", false))
        .unwrap();

    let snapshot = RecoverySnapshot {
        processing_projects: vec![project.clone()],
        linked_jobs: vec![job.clone()],
        active_jobs: vec![],
    };

    let mock = Arc::new(MockRecoveryStorage::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(mock.clone());

    let report = usecase.execute().await.unwrap();
    assert_eq!(report.reconciled_terminal_projects, 1);

    let reconciled = mock.reconciled_projects.lock().await;
    assert_eq!(reconciled.len(), 1);
    assert_eq!(*reconciled[0].status(), ProjectStatus::Failed);
    // Job status should NOT be APP_RESTART, it was untouched!
    let pairs = mock.interrupted_pairs.lock().await;
    assert_eq!(pairs.len(), 0);
}

#[tokio::test]
async fn test_processing_without_job() {
    let project = create_project(ProjectStatus::Processing, None);

    let snapshot = RecoverySnapshot {
        processing_projects: vec![project.clone()],
        linked_jobs: vec![],
        active_jobs: vec![],
    };

    let mock = Arc::new(MockRecoveryStorage::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(mock.clone());

    let report = usecase.execute().await.unwrap();
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(
        report.warnings[0].issue_type,
        RecoveryIssueType::MissingActiveJob
    );

    let failed_no_job = mock.failed_projects_no_job.lock().await;
    assert_eq!(failed_no_job.len(), 1);
    assert_eq!(*failed_no_job[0].status(), ProjectStatus::Failed);
}

#[tokio::test]
async fn test_orphan_active_job() {
    let project = create_project(ProjectStatus::Completed, None);
    let job = create_job(project.id().clone(), JobStatus::Running, None);

    let snapshot = RecoverySnapshot {
        processing_projects: vec![],
        linked_jobs: vec![],
        active_jobs: vec![job.clone()],
    };

    let mock = Arc::new(MockRecoveryStorage::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(mock.clone());

    let report = usecase.execute().await.unwrap();
    assert_eq!(report.recovered_orphan_jobs, 1);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(
        report.warnings[0].issue_type,
        RecoveryIssueType::OrphanActiveJob
    );

    let orphans = mock.orphan_jobs.lock().await;
    assert_eq!(orphans.len(), 1);
    assert_eq!(*orphans[0].status(), JobStatus::Failed);
}

#[tokio::test]
async fn test_job_project_mismatch() {
    let job_id = JobId::new();
    let project = create_project(ProjectStatus::Processing, Some(job_id.clone()));

    let other_project_id = ProjectId::new();
    let job = create_job(
        other_project_id.clone(),
        JobStatus::Running,
        Some(job_id.clone()),
    );

    let snapshot = RecoverySnapshot {
        processing_projects: vec![project.clone()],
        linked_jobs: vec![job.clone()],
        active_jobs: vec![],
    };

    let mock = Arc::new(MockRecoveryStorage::new(snapshot));
    let usecase = RecoverInterruptedStateUseCase::new(mock.clone());

    let report = usecase.execute().await.unwrap();
    assert!(report.has_fatal_issues());
    assert_eq!(
        report.fatal_issues[0].issue_type,
        RecoveryIssueType::JobProjectMismatch
    );
}
