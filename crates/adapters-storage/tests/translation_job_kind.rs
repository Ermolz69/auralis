use adapters_storage::sqlite::{SqliteJobRepository, SqliteProjectRepository, connect_sqlite};
use domain::job::{Job, JobKind};
use domain::project::Project;
use ports::repository::{JobRepository, ProjectRepository};
use std::error::Error;

#[tokio::test]
async fn translation_job_round_trips_without_changing_project_processing_state()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = SqliteProjectRepository::new(pool.clone());
    let project = projects.create(Project::new("Translation".into())?).await?;
    let initial = projects.get(project.id()).await?.ok_or("missing project")?;
    let jobs = SqliteJobRepository::new(pool);
    let mut job = Job::new(
        project.id().clone(),
        "Translate subtitles".into(),
        JobKind::Translation,
    );
    job.start()?;
    let created = jobs.create(job).await?;
    let stored = jobs
        .get(created.id())
        .await?
        .ok_or("missing translation job")?;
    assert_eq!(stored.kind(), &JobKind::Translation);
    assert!(stored.stage().is_none());
    assert_eq!(projects.get(project.id()).await?, Some(initial));
    Ok(())
}
