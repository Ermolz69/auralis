#![allow(clippy::unwrap_used)]
use adapters_storage::sqlite::{SqliteProjectRepository, connect_sqlite};
use application::usecases::project::rename::{RenameProjectRequest, RenameProjectUseCase};
use ports::repository::ProjectRepository;
use std::sync::{Arc, Mutex};
use tracing::Instrument;
use tracing::instrument::WithSubscriber;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt};

#[derive(Clone)]
pub(super) struct Capture(pub(super) Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> MakeWriter<'a> for Capture {
    type Writer = Self;
    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

#[tokio::test]
async fn rename_completion_contains_safe_outcomes_in_json_and_console() {
    for json in [false, true] {
        let capture = Capture(Arc::default());
        let layer = tracing_subscriber::fmt::layer()
            .with_writer(capture.clone())
            .with_ansi(false);
        use tracing_subscriber::Layer;
        let layer = if json {
            layer.json().boxed()
        } else {
            layer.compact().boxed()
        };
        let subscriber = tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new("info"))
            .with(layer);
        let root = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(root.path().join("rename.sqlite"))
            .await
            .unwrap();
        let repo = Arc::new(SqliteProjectRepository::new(pool.clone()));
        let project = repo
            .create(domain::project::Project::new("Original".into()).unwrap())
            .await
            .unwrap();
        let usecase = RenameProjectUseCase::new(repo);
        let dispatch = tracing::Dispatch::new(subscriber);
        let span = tracing::dispatcher::with_default(&dispatch, || {
            tracing::info_span!(
                "rename_project_cmd",
                request_id = "00000000-0000-4000-8000-000000000001"
            )
        });
        for expected_success in [true, false] {
            let result = super::command::observe("rename_project_cmd", async {
                usecase
                    .execute(RenameProjectRequest {
                        project_id: project.id().clone(),
                        expected_revision: project.revision(),
                        title: "SYNTHETIC_PRIVATE_TITLE".into(),
                    })
                    .await
                    .map_err(crate::dto::error::CommandError::from)
            })
            .instrument(span.clone())
            .with_subscriber(dispatch.clone())
            .await;
            assert_eq!(result.is_ok(), expected_success);
        }
        let output = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
        assert_eq!(output.matches("command_completed").count(), 2);
        for expected in [
            "succeeded",
            "conflict",
            "CONFLICT",
            "duration_ms",
            "00000000-0000-4000-8000-000000000001",
        ] {
            assert!(output.contains(expected), "{output}");
        }
        assert!(!output.contains("SYNTHETIC_PRIVATE_TITLE"));
        pool.close().await;
    }
}
