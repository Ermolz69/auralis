use super::Job;
use crate::error::DomainError;
use crate::job::{JobKind, JobProgress, JobStatus};
use chrono::Utc;

impl Job {
    pub fn report_speech_progress(
        &mut self,
        saved_chunks: u64,
        total_chunks: u64,
    ) -> Result<(), DomainError> {
        if self.kind != JobKind::Speech || self.status != JobStatus::Running {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?} {:?}", self.kind, self.status),
                to: "Speech progress".into(),
            });
        }
        if total_chunks == 0
            || saved_chunks > total_chunks
            || self
                .progress
                .total_items
                .is_some_and(|previous| previous != total_chunks)
            || self
                .progress
                .processed_items
                .is_some_and(|previous| saved_chunks < previous)
        {
            return Err(DomainError::ValidationError(
                "Speech checkpoint progress is invalid or moved backwards".into(),
            ));
        }
        let percent = u8::try_from(u128::from(saved_chunks) * 99 / u128::from(total_chunks))
            .map_err(|_| DomainError::ValidationError("Invalid speech percent".into()))?;
        self.bump_revision()?;
        self.progress = JobProgress {
            percent,
            message: format!("Saved {saved_chunks} of {total_chunks} speech chunks"),
            current_step: Some("speech".into()),
            processed_items: Some(saved_chunks),
            total_items: Some(total_chunks),
        };
        self.updated_at = Utc::now();
        Ok(())
    }
}
