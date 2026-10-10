use adapters_storage::sqlite::{SqliteJobRepository, SqliteProjectRepository, connect_sqlite};
use domain::job::{Job, JobKind};
use domain::project::Project;
use ports::repository::{JobRepository, ProjectRepository};
use std::error::Error;

#[tokio::test]
async fn saved_speech_progress_round_trips_without_mutating_the_project()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let projects = SqliteProjectRepository::new(pool.clone());
    let project = projects.create(Project::new("Speech".into())?).await?;
    let initial = projects.get(project.id()).await?.ok_or("missing project")?;
    let jobs = SqliteJobRepository::new(pool);
    let mut job = Job::new(
        project.id().clone(),
        "Synthesize speech".into(),
        JobKind::Speech,
    );
    job.start()?;
    job.report_speech_progress(2, 5)?;
    let created = jobs.create(job).await?;
    let stored = jobs.get(created.id()).await?.ok_or("missing speech job")?;
    assert_eq!(stored.kind(), &JobKind::Speech);
    assert_eq!(stored.progress().processed_items, Some(2));
    assert_eq!(stored.progress().total_items, Some(5));
    assert!(stored.stage().is_none());
    assert_eq!(projects.get(project.id()).await?, Some(initial));
    Ok(())
}
