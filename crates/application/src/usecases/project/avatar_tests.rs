#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::avatar::ProjectAvatarUseCase;
use adapters_storage::sqlite::{
    SqliteProjectAvatarRepository, SqliteProjectRepository, connect_sqlite,
};
use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;
use std::sync::Arc;

#[tokio::test]
async fn validates_avatar_before_persistence_and_supports_read_clear_and_migration() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(dir.path().join("avatars.sqlite"))
        .await
        .unwrap();
    let project = Project::new("Project".into()).unwrap();
    SqliteProjectRepository::new(pool.clone())
        .create(project.clone())
        .await
        .unwrap();
    let usecase = ProjectAvatarUseCase::new(Arc::new(SqliteProjectAvatarRepository::new(pool)));
    let id = project.id().clone();
    assert!(!usecase.get(id.clone()).await.unwrap().initialized);
    assert!(
        usecase
            .set(
                id.clone(),
                Some("data:image/svg+xml;base64,PHN2Zz4=".into()),
                false
            )
            .await
            .is_err()
    );
    assert!(!usecase.get(id.clone()).await.unwrap().initialized);
    let data_url = "data:image/png;base64,iVBORw0KGgo=".to_string();
    let saved = usecase
        .set(id.clone(), Some(data_url.clone()), false)
        .await
        .unwrap();
    assert_eq!(saved.avatar.unwrap().as_str(), data_url);
    usecase.set(id.clone(), None, false).await.unwrap();
    let migrated = usecase.set(id, Some(data_url), true).await.unwrap();
    assert!(migrated.initialized);
    assert!(migrated.avatar.is_none());
    assert!(usecase.get(ProjectId::new()).await.is_err());
}
