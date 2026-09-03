#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{
    create_from_youtube::CreateProjectFromYoutubeRequest, youtube_atomic_support::Fixture,
};
use domain::project::Project;
use ports::repository::ProjectRepository;

#[tokio::test]
async fn a_commit_time_failure_rolls_back_new_and_existing_imports_and_allows_retry() {
    for existing in [false, true] {
        let fixture = Fixture::new().await;
        let original = if existing {
            Some(
                fixture
                    .repo
                    .create(Project::new("Keep".into()).unwrap())
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        let id = original.as_ref().map(|project| project.id().clone());
        sqlx::raw_sql("CREATE TABLE commit_guard (project_id TEXT REFERENCES projects(id) DEFERRABLE INITIALLY DEFERRED);
            CREATE TRIGGER reject_commit AFTER INSERT ON outbox_messages BEGIN INSERT INTO commit_guard VALUES ('missing-project'); END;")
            .execute(&fixture.pool).await.unwrap();
        let result = fixture
            .usecase()
            .execute(CreateProjectFromYoutubeRequest {
                url: "https://youtube.com/watch?v=commit".into(),
                project_id: id.clone(),
            })
            .await;
        let error = result
            .err()
            .expect("deferred foreign key must reject COMMIT");
        assert!(
            matches!(
                &error,
                crate::error::ApplicationError::Port(ports::error::PortError::Storage {
                    operation: "commit_youtube_import",
                    ..
                })
            ),
            "{error:?}"
        );
        fixture.assert_unchanged(original.as_ref()).await;
        sqlx::raw_sql("DROP TRIGGER reject_commit; DROP TABLE commit_guard;")
            .execute(&fixture.pool)
            .await
            .unwrap();
        fixture
            .usecase()
            .execute(CreateProjectFromYoutubeRequest {
                url: "https://youtube.com/watch?v=commit".into(),
                project_id: id,
            })
            .await
            .unwrap();
        fixture.pool.close().await;
    }
}
