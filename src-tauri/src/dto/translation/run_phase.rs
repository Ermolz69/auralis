use ports::translation_run_control::TranslationRunPhase;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationRunPhaseDto {
    Requested,
    Running,
    Paused,
    Failed,
    Validated,
}

impl From<TranslationRunPhase> for TranslationRunPhaseDto {
    fn from(value: TranslationRunPhase) -> Self {
        match value {
            TranslationRunPhase::Requested => Self::Requested,
            TranslationRunPhase::Running => Self::Running,
            TranslationRunPhase::Paused => Self::Paused,
            TranslationRunPhase::Failed => Self::Failed,
            TranslationRunPhase::Validated => Self::Validated,
        }
    }
}
