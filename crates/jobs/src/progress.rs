use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub percent: u8,
}

impl JobProgress {
    pub fn new(percent: u8) -> Self {
        Self { percent }
    }
}

impl Default for JobProgress {
    fn default() -> Self {
        Self::new(0)
    }
}
