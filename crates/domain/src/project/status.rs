#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProjectStatus {
    Draft,
    SourceImported,
    ReadyForProcessing,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalTransitionResult {
    Applied { transcript_ready: bool },
    AlreadyApplied,
    IgnoredStale,
    ProjectMissing,
}
