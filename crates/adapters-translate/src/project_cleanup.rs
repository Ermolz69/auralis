use async_trait::async_trait;
use auralis_translation::TranslationId;
use auralis_translation_sqlite::{DbError, SqliteConfig, TranslateDb};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::translation_cleanup::TranslationCleanupPort;
use std::path::PathBuf;
use uuid::Uuid;

pub struct TranslateProjectCleanup {
    database_path: PathBuf,
}

impl TranslateProjectCleanup {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

#[async_trait]
impl TranslationCleanupPort for TranslateProjectCleanup {
    async fn delete_project_translation(
        &self,
        project_id: &ProjectId,
        translation_id: Uuid,
    ) -> Result<(), PortError> {
        let database_path = self.database_path.clone();
        let project_id = project_id.to_string();
        tokio::task::spawn_blocking(move || {
            if !database_path.is_file() {
                return Err(PortError::NotFound {
                    resource: "Translate database".into(),
                });
            }
            let translation_id =
                TranslationId::new(translation_id).ok_or_else(|| PortError::InvalidSource {
                    message: "translation ID is nil".into(),
                })?;
            let mut database = TranslateDb::open(&database_path, SqliteConfig::default())
                .map_err(|error| map_database_error("open_translate_database", error))?;
            database
                .delete_project_translation(translation_id, &project_id)
                .map_err(|error| map_database_error("delete_project_translation", error))?;
            Ok(())
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
}

fn map_database_error(operation: &'static str, error: DbError) -> PortError {
    match error {
        DbError::Conflict(reason) => PortError::Conflict {
            resource: "project translation".into(),
            message: reason.into(),
        },
        other => PortError::Storage {
            operation,
            message: other.to_string(),
        },
    }
}
