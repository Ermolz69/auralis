use std::sync::Arc;

use crate::error::ApplicationError;
use domain::media::ArtifactLocation;
use domain::outbox::{OutboxMessage, OutboxPayload};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::transaction::{TransactionGateway, UnitOfWorkData};

pub struct DeleteProjectRequest {
    pub project_id: ProjectId,
}

pub struct DeleteProjectUseCase {
    artifact_index: Arc<dyn ArtifactIndex>,
    transaction_gateway: Arc<dyn TransactionGateway>,
}

impl DeleteProjectUseCase {
    pub fn new(
        artifact_index: Arc<dyn ArtifactIndex>,
        transaction_gateway: Arc<dyn TransactionGateway>,
    ) -> Self {
        Self {
            artifact_index,
            transaction_gateway,
        }
    }

    pub async fn execute(&self, request: DeleteProjectRequest) -> Result<(), ApplicationError> {
        let project_id = &request.project_id;

        // 1. List all artifacts for the project
        let artifacts = self.artifact_index.list_by_project(project_id).await?;

        // 2. Prepare UnitOfWorkData
        let mut uow = UnitOfWorkData::new();

        // 3. For each StorageKey artifact, schedule deletion
        for artifact in artifacts {
            if let ArtifactLocation::StorageKey(storage_key) = artifact.location {
                uow = uow.add_outbox_message(OutboxMessage::new(OutboxPayload::DeleteStorageKey {
                    storage_key,
                }));
            }
        }

        // 4. Schedule project directory deletion
        uow = uow.add_outbox_message(OutboxMessage::new(
            OutboxPayload::DeleteProjectArtifactDir {
                project_id: project_id.clone(),
            },
        ));

        // 5. Delete project (this will cascade delete artifacts via SQLite)
        uow = uow.delete_project(project_id.clone());

        // 6. Execute transaction
        self.transaction_gateway.execute(uow).await.map_err(|e| {
            ApplicationError::InvalidOperation {
                message: format!("Failed to delete project: {}", e),
            }
        })?;

        Ok(())
    }
}
