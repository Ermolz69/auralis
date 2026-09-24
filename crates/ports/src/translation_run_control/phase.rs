#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationRunPhase {
    Requested,
    Running,
    Paused,
    Failed,
    Validated,
}
