use domain::project::{Project, ProjectStatus};
use ports::{
    error::PortError,
    transaction::CommitYoutubeImport,
    youtube_import::{YoutubeImportSession, YoutubeImportState},
};

pub(super) fn invalid() -> PortError {
    PortError::InvalidStoredData {
        entity_type: "youtube_import".into(),
        entity_id: "journal".into(),
        field: "payload_json".into(),
        message: "Invalid YouTube import checkpoint".into(),
    }
}

pub(super) fn validate(session: &YoutubeImportSession) -> Result<(), PortError> {
    let project = Project::from_snapshot(session.project.clone()).map_err(|_| invalid())?;
    if session.url().is_empty()
        || project.status() != &ProjectStatus::ReadyForProcessing
        || project.metadata().is_none()
        || project.active_job_id().is_some()
        || project.last_terminal_job_id().is_some()
        || project.transcript().is_some()
        || (session.original_updated_at.is_none() && project.revision() != 1)
        || (session.state == YoutubeImportState::Staged && session.write.is_none())
        || (session.state == YoutubeImportState::Downloading && session.write.is_some())
        || session.revision == 0
        || session.revision >= i64::MAX as u64
        || !session
            .workspace_key
            .as_str()
            .starts_with(&format!("tmp/{}/youtube-resume_", session.project.id))
        || session.workspace_key.as_str().split('/').count() != 3
    {
        return Err(invalid());
    }
    if let Some(write) = &session.write {
        write.validate()?;
        CommitYoutubeImport {
            project,
            write: write.clone(),
            original_updated_at: session.original_updated_at,
        }
        .validate()?;
        if write.project_id != session.project.id
            || write.temp_workspace_key.as_ref() != Some(&session.workspace_key)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn decode(
    (id, key, revision, payload): (String, String, i64, String),
) -> Result<YoutubeImportSession, PortError> {
    let session: YoutubeImportSession = serde_json::from_str(&payload).map_err(|_| invalid())?;
    validate(&session)?;
    if session.project.id.to_string() != id
        || session.request_key() != key
        || session.revision != revision as u64
    {
        return Err(invalid());
    }
    Ok(session)
}

pub(super) fn conflict() -> PortError {
    PortError::Conflict {
        resource: "YouTube import".into(),
        message: "Import changed, was cancelled, or is already running".into(),
    }
}
