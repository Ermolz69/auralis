//! In-memory storage test doubles.
//!
//! These adapters exist for application and adapter tests that need cheap
//! repositories without durable SQLite setup. Desktop production wiring must not
//! select them: SQLite is the only production source of truth, and the memory
//! unit of work intentionally does not provide outbox or crash-durable parity.

pub mod artifact_index;
pub mod database;
pub mod job_repository;
pub mod project_repository;
mod project_writes;
pub mod recovery_gateway;
pub mod unit_of_work;
#[cfg(test)]
mod unit_of_work_tests;

pub use artifact_index::InMemoryArtifactIndex;
pub use database::InMemoryDatabase;
pub use job_repository::InMemoryJobRepository;
pub use project_repository::InMemoryProjectRepository;
pub use unit_of_work::InMemoryStorageUnitOfWork;
