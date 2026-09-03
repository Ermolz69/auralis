use super::setup_db;
use crate::sqlite::{SqliteJobRepository, SqliteProjectRepository, SqliteStorageUnitOfWork};
use domain::{
    job::{Job, JobKind},
    media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState, MediaSource},
    project::Project,
    transcript::Transcript,
};
use ports::{
    error::PortError,
    project_update::ProjectUpdate,
    repository::{JobRepository, ProjectRepository},
    transaction::{
        ApplyTerminalLifecycle, CommitPipelineStart, CommitTranscriptImport, StorageUnitOfWork,
    },
};

fn ready_project() -> Project {
    let mut project = Project::new("Original".into());
    project
        .import_source(
            MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=current".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    project
}

fn transcript_command(mut project: Project) -> CommitTranscriptImport {
    let expected_project_updated_at = project.updated_at();
    let expected_status = project.status().clone();
    let expected_active_job_id = project.active_job_id().cloned();
    project.set_transcript(Transcript {
        language: "en".into(),
        segments: vec![],
    });
    let artifact_id = ArtifactId::new();
    let final_key = format!("{}/original-subtitle/{artifact_id}.vtt", project.id());
    CommitTranscriptImport {
        artifact: Artifact {
            id: artifact_id,
            kind: ArtifactKind::OriginalSubtitle,
            location: ArtifactLocation::StorageKey(final_key.clone()),
            size_bytes: Some(1),
            state: ArtifactState::PendingFinalize,
            created_at: project.updated_at(),
            updated_at: project.updated_at(),
            ready_at: None,
        },
        staging_key: format!(".staging/{}/subtitle.vtt", project.id()),
        final_key,
        project,
        temp_workspace_key: None,
        expected_project_updated_at,
        expected_status,
        expected_active_job_id,
    }
}

#[tokio::test]
async fn stale_pipeline_start_cannot_overwrite_rename_even_with_identical_timestamps() {
    let pool = setup_db().await;
    let repo = SqliteProjectRepository::new(pool.clone());
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let mut project = repo.create(ready_project()).await.unwrap();
    let renamed = repo
        .update(
            project.id(),
            project.revision(),
            ProjectUpdate::Rename {
                title: "Current title".into(),
            },
            project.updated_at(),
        )
        .await
        .unwrap();
    let job = Job::new(
        project.id().clone(),
        project.title().into(),
        JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    assert!(matches!(
        uow.commit_pipeline_start(CommitPipelineStart {
            project: project.clone(),
            job
        })
        .await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), renamed);
    let jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs, 0);
}

#[tokio::test]
async fn pipeline_insert_failure_rolls_back_project_revision() {
    let pool = setup_db().await;
    let repo = SqliteProjectRepository::new(pool.clone());
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let original = repo.create(ready_project()).await.unwrap();
    let job = Job::new(
        original.id().clone(),
        original.title().into(),
        JobKind::Dubbing,
    );
    SqliteJobRepository::new(pool)
        .create(job.clone())
        .await
        .unwrap();
    let mut project = original.clone();
    project.start_processing(job.id().clone()).unwrap();
    assert!(
        uow.commit_pipeline_start(CommitPipelineStart { project, job })
            .await
            .is_err()
    );
    assert_eq!(repo.get(original.id()).await.unwrap().unwrap(), original);
}

#[tokio::test]
async fn transcript_and_terminal_writes_invalidate_stale_renames() {
    let pool = setup_db().await;
    let repo = SqliteProjectRepository::new(pool.clone());
    let uow = SqliteStorageUnitOfWork::new(pool);
    let mut project = repo.create(ready_project()).await.unwrap();
    let job = Job::new(
        project.id().clone(),
        project.title().into(),
        JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    uow.commit_pipeline_start(CommitPipelineStart {
        project: project.clone(),
        job: job.clone(),
    })
    .await
    .unwrap();
    let processing = repo.get(project.id()).await.unwrap().unwrap();
    uow.commit_transcript_import(transcript_command(processing.clone()))
        .await
        .unwrap();
    let transcribed = repo.get(project.id()).await.unwrap().unwrap();
    assert_eq!(transcribed.revision(), processing.revision() + 1);
    assert!(matches!(
        repo.update(
            project.id(),
            processing.revision(),
            ProjectUpdate::Rename {
                title: "Stale".into()
            },
            processing.updated_at()
        )
        .await,
        Err(PortError::Conflict { .. })
    ));
    let command = ApplyTerminalLifecycle {
        project_id: project.id().clone(),
        job_id: job.id().clone(),
        outcome: domain::job::TerminalOutcome::Completed,
    };
    uow.apply_terminal_lifecycle_conditionally(command)
        .await
        .unwrap();
    let completed = repo.get(project.id()).await.unwrap().unwrap();
    assert_eq!(completed.revision(), transcribed.revision() + 1);
    assert_eq!(completed.transcript(), transcribed.transcript());
    assert!(matches!(
        repo.update(
            project.id(),
            transcribed.revision(),
            ProjectUpdate::Rename {
                title: "Stale".into()
            },
            transcribed.updated_at()
        )
        .await,
        Err(PortError::Conflict { .. })
    ));
    uow.apply_terminal_lifecycle_conditionally(ApplyTerminalLifecycle {
        project_id: project.id().clone(),
        job_id: job.id().clone(),
        outcome: domain::job::TerminalOutcome::Completed,
    })
    .await
    .unwrap();
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), completed);
}

#[tokio::test]
async fn stale_transcript_commit_rolls_back_artifacts_and_outbox_after_rename() {
    let pool = setup_db().await;
    let repo = SqliteProjectRepository::new(pool.clone());
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let mut project = repo.create(ready_project()).await.unwrap();
    let job = Job::new(
        project.id().clone(),
        project.title().into(),
        JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    uow.commit_pipeline_start(CommitPipelineStart {
        project: project.clone(),
        job,
    })
    .await
    .unwrap();
    let processing = repo.get(project.id()).await.unwrap().unwrap();
    let renamed = repo
        .update(
            project.id(),
            processing.revision(),
            ProjectUpdate::Rename {
                title: "Current title".into(),
            },
            processing.updated_at(),
        )
        .await
        .unwrap();
    assert!(matches!(
        uow.commit_transcript_import(transcript_command(processing))
            .await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), renamed);
    for table in ["artifacts", "outbox_messages"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
