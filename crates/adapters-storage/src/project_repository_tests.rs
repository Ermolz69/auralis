#![allow(clippy::unwrap_used)]
use crate::{
    memory::{InMemoryDatabase, InMemoryProjectRepository},
    sqlite::{SqliteProjectRepository, connect_sqlite},
};
use domain::{
    job::JobId,
    media::MediaSource,
    project::{LanguageCode, Project, ProjectStatus},
    transcript::Transcript,
};
use ports::{error::PortError, project_update::ProjectUpdate, repository::ProjectRepository};
use std::sync::{Arc, Mutex};

async fn repository_contract(repo: &dyn ProjectRepository) {
    let mut project = Project::new("Original".into()).unwrap();
    project.set_languages(
        Some(LanguageCode("en".into())),
        Some(LanguageCode("ru".into())),
    );
    project.set_transcript(Transcript {
        language: "en".into(),
        segments: vec![],
    });
    let mut snapshot = project.to_snapshot();
    snapshot.last_terminal_job_id = Some(JobId::new());
    let project = repo
        .create(Project::from_snapshot(snapshot).unwrap())
        .await
        .unwrap();
    assert_eq!(project.revision(), 1);
    let timestamp = project.updated_at();
    for title in ["", " \t\n", "\u{a0}\u{2003}"] {
        assert!(matches!(
            repo.update(
                project.id(),
                project.revision(),
                ProjectUpdate::Rename {
                    title: title.into()
                },
                timestamp,
            )
            .await,
            Err(PortError::Conflict { .. })
        ));
        assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), project);
    }
    let rename = ProjectUpdate::Rename {
        title: " \tRenamed\u{a0}".into(),
    };
    let (first, second) = tokio::join!(
        repo.update(project.id(), 1, rename.clone(), timestamp),
        repo.update(project.id(), 1, rename.clone(), timestamp),
    );
    let renamed = match (first, second) {
        (Ok(project), Err(PortError::Conflict { .. }))
        | (Err(PortError::Conflict { .. }), Ok(project)) => project,
        other => panic!("Expected exactly one successful rename: {other:?}"),
    };
    let mut expected = project.to_snapshot();
    expected.title = "Renamed".into();
    expected.revision = 2;
    assert_eq!(renamed.to_snapshot(), expected);
    assert_eq!(repo.list().await.unwrap(), vec![renamed.clone()]);

    let source = MediaSource::YoutubeUrl {
        url: "https://youtube.com/watch?v=current".into(),
    };
    let import = ProjectUpdate::ImportSource {
        source: source.clone(),
        metadata: None,
    };
    assert!(matches!(
        repo.update(project.id(), 1, import.clone(), timestamp)
            .await,
        Err(PortError::Conflict { .. })
    ));
    let imported = repo
        .update(project.id(), 2, import.clone(), timestamp)
        .await
        .unwrap();
    expected.revision = 3;
    expected.status = ProjectStatus::SourceImported;
    expected.source = Some(source);
    assert_eq!(imported.to_snapshot(), expected);
    assert!(matches!(
        repo.update(project.id(), 3, import.clone(), timestamp)
            .await,
        Err(PortError::Conflict { .. })
    ));
    assert!(matches!(
        repo.update(
            project.id(),
            2,
            ProjectUpdate::MarkReadyForProcessing,
            timestamp
        )
        .await,
        Err(PortError::Conflict { .. })
    ));
    let ready = repo
        .update(
            project.id(),
            3,
            ProjectUpdate::MarkReadyForProcessing,
            timestamp,
        )
        .await
        .unwrap();
    expected.revision = 4;
    expected.status = ProjectStatus::ReadyForProcessing;
    assert_eq!(ready.to_snapshot(), expected);
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), ready);

    repo.delete(project.id()).await.unwrap();
    for update in [rename, import, ProjectUpdate::MarkReadyForProcessing] {
        assert!(matches!(
            repo.update(project.id(), 4, update, timestamp).await,
            Err(PortError::NotFound { .. })
        ));
    }
    assert!(repo.get(project.id()).await.unwrap().is_none());

    let mut snapshot = Project::new("Maximum revision".into())
        .unwrap()
        .to_snapshot();
    snapshot.revision = i64::MAX as u64;
    let maximum = repo
        .create(Project::from_snapshot(snapshot).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        repo.update(
            maximum.id(),
            maximum.revision(),
            ProjectUpdate::Rename {
                title: "Overflow".into()
            },
            timestamp
        )
        .await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(repo.get(maximum.id()).await.unwrap().unwrap(), maximum);
}

#[tokio::test]
async fn sqlite_project_updates_are_scoped_and_revision_guarded() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(dir.path().join("projects.sqlite"))
        .await
        .unwrap();
    repository_contract(&SqliteProjectRepository::new(pool)).await;
}

#[tokio::test]
async fn memory_project_updates_match_sqlite_contract() {
    repository_contract(&InMemoryProjectRepository::new(Arc::new(Mutex::new(
        InMemoryDatabase::new(),
    ))))
    .await;
}
