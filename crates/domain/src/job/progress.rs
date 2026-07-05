use crate::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobProgress {
    pub percent: u8,
    pub message: String,
    pub current_step: Option<String>,
    pub processed_items: Option<u64>,
    pub total_items: Option<u64>,
}

impl JobProgress {
    pub fn initializing() -> Self {
        Self {
            percent: 0,
            message: "Initializing...".to_string(),
            current_step: None,
            processed_items: None,
            total_items: None,
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.percent > 100 {
            return Err(DomainError::ValidationError(
                "Job progress percent cannot be greater than 100".to_string(),
            ));
        }

        if let (Some(processed), Some(total)) = (self.processed_items, self.total_items) {
            if processed > total {
                return Err(DomainError::ValidationError(
                    "Processed items cannot be greater than total items".to_string(),
                ));
            }
        }

        Ok(())
    }
}
