use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(try_from = "u64", into = "u64")]
pub struct SpeechExecutionBudget(u64);

impl SpeechExecutionBudget {
    pub const DEFAULT_SECONDS: u64 = 45 * 60;
    pub const MAX_SECONDS: u64 = 48 * 60 * 60;

    pub fn seconds(self) -> u64 {
        self.0
    }

    pub fn duration(self) -> Duration {
        Duration::from_secs(self.0)
    }
}

impl Default for SpeechExecutionBudget {
    fn default() -> Self {
        Self(Self::DEFAULT_SECONDS)
    }
}

impl TryFrom<u64> for SpeechExecutionBudget {
    type Error = String;

    fn try_from(seconds: u64) -> Result<Self, Self::Error> {
        if !(1..=Self::MAX_SECONDS).contains(&seconds) {
            return Err(format!(
                "Speech execution budget must be 1..={} seconds",
                Self::MAX_SECONDS
            ));
        }
        Ok(Self(seconds))
    }
}

impl From<SpeechExecutionBudget> for u64 {
    fn from(budget: SpeechExecutionBudget) -> Self {
        budget.0
    }
}
