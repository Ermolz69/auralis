use crate::TracingShutdownOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TracingShutdownReport {
    pub file: TracingShutdownOutcome,
    pub console: TracingShutdownOutcome,
    pub sampler: TracingShutdownOutcome,
}

impl TracingShutdownReport {
    pub fn not_owned() -> Self {
        Self {
            file: TracingShutdownOutcome::NotOwned,
            console: TracingShutdownOutcome::NotOwned,
            sampler: TracingShutdownOutcome::NotOwned,
        }
    }
    pub fn outcome(&self) -> TracingShutdownOutcome {
        let outcomes = [self.file, self.console, self.sampler];
        for outcome in outcomes {
            if !outcome.is_graceful() {
                return outcome;
            }
        }
        if outcomes.contains(&TracingShutdownOutcome::Flushed) {
            TracingShutdownOutcome::Flushed
        } else {
            TracingShutdownOutcome::NotOwned
        }
    }
}
