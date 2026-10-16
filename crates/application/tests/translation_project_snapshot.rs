#[path = "support/gated_translation_runtime.rs"]
#[expect(dead_code)]
mod gated_translation_runtime;
#[path = "support/translation_admission_fixture.rs"]
#[expect(dead_code)]
mod translation_admission_fixture;

use adapters_storage::sqlite::SqliteProjectRepository;
use adapters_translate::TranslateRunControl;
use application::error::ApplicationError;
use application::usecases::translation::{
    GetProjectTranslationSnapshotUseCase, GetTranslationRunStatusUseCase,
    ListProjectTranslationsUseCase, TranslationRunUseCase,
};
use async_trait::async_trait;
use domain::job::{Job, JobKind};
use domain::project::{Project, ProjectId};
use gated_translation_runtime::GatedTranslationRuntime;
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_links::{TranslationLink, TranslationLinkStore};
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunSnapshot};
use ports::translation_start_guard::TranslationStartGuard;
use sqlx::SqlitePool;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use translation_admission_fixture::TranslationAdmissionFixture;
use uuid::Uuid;

struct RevisionChangingControl {
    inner: TranslateRunControl,
    pool: SqlitePool,
    remaining_changes: AtomicUsize,
}

#[async_trait]
impl TranslationRunControlPort for RevisionChangingControl {
    async fn capture_start(
        &self,
        link: &TranslationLink,
    ) -> Result<TranslationStartGuard, PortError> {
        self.inner.capture_start(link).await
    }

    async fn snapshot(&self, link: &TranslationLink) -> Result<TranslationRunSnapshot, PortError> {
        let previous =
            self.remaining_changes
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                    remaining.checked_sub(1)
                });
        if previous.is_ok() {
            sqlx::query(
                "UPDATE project_translations SET revision = revision + 1 WHERE translation_id = ?",
            )
            .bind(link.translation_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|error| PortError::Storage {
                operation: "advance_translation_link_revision",
                message: error.to_string(),
            })?;
        }
        self.inner.snapshot(link).await
    }

    async fn snapshot_for_run(
        &self,
        link: &TranslationLink,
        run_id: Uuid,
    ) -> Result<TranslationRunSnapshot, PortError> {
        self.inner.snapshot_for_run(link, run_id).await
    }

    async fn request_pause(&self, link: &TranslationLink) -> Result<(), PortError> {
        self.inner.request_pause(link).await
    }
}

fn usecase(
    fixture: &TranslationAdmissionFixture,
    revision_changes: usize,
) -> GetProjectTranslationSnapshotUseCase {
    let projects = Arc::new(SqliteProjectRepository::new(fixture.pool.clone()));
    let control = Arc::new(RevisionChangingControl {
        inner: TranslateRunControl::new(fixture.translate_path.clone()),
        pool: fixture.pool.clone(),
        remaining_changes: AtomicUsize::new(revision_changes),
    });
    GetProjectTranslationSnapshotUseCase::new(
        ListProjectTranslationsUseCase::new(projects, fixture.links.clone()),
        GetTranslationRunStatusUseCase::new(
            TranslationRunUseCase::new(fixture.links.clone(), control),
            fixture.jobs.clone(),
        ),
        fixture.links.clone(),
    )
}

#[tokio::test]
async fn project_snapshot_reads_owned_committed_run_and_host_job_from_both_databases()
-> Result<(), Box<dyn Error>> {
    let fixture = TranslationAdmissionFixture::new(GatedTranslationRuntime::new(
        "http://127.0.0.1:9/".into(),
    ))
    .await?;
    let job = fixture
        .jobs
        .create_pending(
            Job::new(
                fixture.request.project_id.clone(),
                "Snapshot preparation".into(),
                JobKind::Translation,
            ),
            fixture.request.translation_id,
            fixture.run_id,
        )
        .await?;
    let read = usecase(&fixture, 0);
    let observed = read.execute(&fixture.request.project_id).await?;
    assert_eq!(observed.project_id, fixture.request.project_id);
    assert_eq!(observed.entries.len(), 1);
    assert_eq!(
        observed.entries[0].link.translation_id,
        fixture.request.translation_id
    );
    let run = observed.entries[0]
        .run
        .as_ref()
        .ok_or("missing active run")?;
    assert_eq!(run.snapshot.run_id, fixture.run_id);
    assert_eq!(run.snapshot.saved_blocks, 0);
    assert_eq!(run.active_host_job_id.as_ref(), Some(job.id()));

    let projects = SqliteProjectRepository::new(fixture.pool.clone());
    let other = projects
        .create(Project::new("Other project".into())?)
        .await?;
    assert!(read.execute(other.id()).await?.entries.is_empty());
    assert!(matches!(
        read.execute(&ProjectId::new()).await,
        Err(ApplicationError::ProjectNotFound(_))
    ));
    Ok(())
}

#[tokio::test]
async fn project_snapshot_retries_changed_links_once_then_reports_conflict()
-> Result<(), Box<dyn Error>> {
    let fixture = TranslationAdmissionFixture::new(GatedTranslationRuntime::new(
        "http://127.0.0.1:9/".into(),
    ))
    .await?;
    let initial_revision = fixture
        .links
        .get(fixture.request.translation_id)
        .await?
        .ok_or("link")?
        .revision;
    let observed = usecase(&fixture, 1)
        .execute(&fixture.request.project_id)
        .await?;
    assert_eq!(observed.entries[0].link.revision, initial_revision + 1);
    assert!(matches!(
        usecase(&fixture, 2)
            .execute(&fixture.request.project_id)
            .await,
        Err(ApplicationError::Port(PortError::Conflict { .. }))
    ));
    Ok(())
}
