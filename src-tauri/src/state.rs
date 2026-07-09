use ports::repository::ProjectRepository;
use std::sync::Arc;

pub type RuntimeProjectRepository = Arc<dyn ProjectRepository>;
