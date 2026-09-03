#![allow(clippy::unwrap_used)]

use super::create::{CreateProjectRequest, CreateProjectUseCase};
use crate::error::ApplicationError;
use adapters_storage::memory::{InMemoryDatabase, InMemoryProjectRepository};
use domain::{error::DomainError, project::ProjectStatus};
use ports::repository::ProjectRepository;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn creates_and_persists_project_with_normalized_title() {
    let repo = InMemoryProjectRepository::new(Arc::new(Mutex::new(InMemoryDatabase::new())));
    let use_case = CreateProjectUseCase::new(repo.clone());

    let response = use_case
        .execute(CreateProjectRequest {
            title: "\u{a0} Test Project \t\n".into(),
        })
        .await
        .unwrap();

    assert_eq!(response.project.title(), "Test Project");
    assert_eq!(*response.project.status(), ProjectStatus::Draft);
    let saved = repo.get(response.project.id()).await.unwrap().unwrap();
    assert_eq!(saved, response.project);
}

#[tokio::test]
async fn rejects_blank_titles_without_persisting_projects() {
    let repo = InMemoryProjectRepository::new(Arc::new(Mutex::new(InMemoryDatabase::new())));
    let use_case = CreateProjectUseCase::new(repo.clone());

    for title in ["", "   ", "\t\r\n", "\u{a0}\u{2003}\u{3000}"] {
        let result = use_case
            .execute(CreateProjectRequest {
                title: title.into(),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DomainError::ValidationError(_)))
        ));
        assert!(repo.list().await.unwrap().is_empty());
    }
}
