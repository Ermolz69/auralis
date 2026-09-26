use std::time::Duration;

#[derive(Clone, Copy)]
pub struct TranslationAdmissionPolicy {
    poll_interval: Duration,
}

impl TranslationAdmissionPolicy {
    pub fn new(poll_interval: Duration) -> Option<Self> {
        (Duration::from_millis(10)..=Duration::from_secs(1))
            .contains(&poll_interval)
            .then_some(Self { poll_interval })
    }

    pub fn poll_interval(self) -> Duration {
        self.poll_interval
    }
}

impl Default for TranslationAdmissionPolicy {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(250),
        }
    }
}
