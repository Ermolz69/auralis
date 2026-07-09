use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::transaction::TransactionGateway;
use std::sync::Arc;

pub type RuntimeProjectRepository = Arc<dyn ProjectRepository>;
pub type RuntimeArtifactIndex = Arc<dyn ArtifactIndex>;
pub type RuntimeArtifactStore = Arc<dyn ArtifactStore>;
pub type RuntimeTransactionGateway = Arc<dyn TransactionGateway>;
