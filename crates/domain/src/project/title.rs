use crate::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectTitle(String);

impl ProjectTitle {
    pub fn new(title: String) -> Result<Self, DomainError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(DomainError::ValidationError(
                "Project title cannot be empty".to_string(),
            ));
        }
        Ok(Self(title.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
