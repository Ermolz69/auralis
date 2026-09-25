use super::Job;
use crate::error::DomainError;
use crate::job::{JobKind, JobProgress, JobStatus};
use chrono::Utc;

impl Job {
    pub fn report_translation_progress(
        &mut self,
        saved_blocks: u64,
        total_blocks: u64,
    ) -> Result<(), DomainError> {
        if self.kind != JobKind::Translation || self.status != JobStatus::Running {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?} {:?}", self.kind, self.status),
                to: "Translation progress".into(),
            });
        }
        if total_blocks == 0
            || saved_blocks > total_blocks
            || self
                .progress
                .total_items
                .is_some_and(|previous| previous != total_blocks)
            || self
                .progress
                .processed_items
                .is_some_and(|previous| saved_blocks < previous)
        {
            return Err(DomainError::ValidationError(
                "Translation checkpoint progress is invalid or moved backwards".into(),
            ));
        }
        let percent = u8::try_from(u128::from(saved_blocks) * 99 / u128::from(total_blocks))
            .map_err(|_| DomainError::ValidationError("Invalid translation percent".into()))?;
        self.bump_revision()?;
        self.progress = JobProgress {
            percent,
            message: format!("Saved {saved_blocks} of {total_blocks} translation blocks"),
            current_step: Some("translation".into()),
            processed_items: Some(saved_blocks),
            total_items: Some(total_blocks),
        };
        self.updated_at = Utc::now();
        Ok(())
    }
}
