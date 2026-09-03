pub mod avatar;
pub mod entity;
pub mod id;
pub mod snapshot;
pub mod status;
pub mod title;

#[cfg(test)]
mod avatar_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod title_tests;

pub use entity::{LanguageCode, Project};
pub use id::ProjectId;
pub use snapshot::ProjectSnapshot;
pub use status::ProjectStatus;
pub use title::ProjectTitle;
