use std::fmt;

#[derive(Default, Debug, Clone)]
pub struct OutboxBatchReport {
    pub fetched: usize,
    pub corrupted: usize,
    pub claimed: usize,
    pub claim_conflicts: usize,
    pub completed: usize,
    pub retry_scheduled: usize,
    pub dead: usize,
    pub storage_errors: usize,
}

impl fmt::Display for OutboxBatchReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OutboxBatchReport {{ fetched: {}, corrupted: {}, claimed: {}, conflicts: {}, completed: {}, retry: {}, dead: {}, errors: {} }}",
            self.fetched,
            self.corrupted,
            self.claimed,
            self.claim_conflicts,
            self.completed,
            self.retry_scheduled,
            self.dead,
            self.storage_errors
        )
    }
}
