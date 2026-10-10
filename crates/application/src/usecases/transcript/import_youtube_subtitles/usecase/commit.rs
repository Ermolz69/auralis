use super::*;

impl ImportYoutubeSubtitlesUseCase {
    pub(super) async fn commit_staged_transcript(
        &self,
        project_id: &domain::project::ProjectId,
        job_id: &domain::job::JobId,
        source: &domain::media::MediaSource,
        transcript: &domain::transcript::Transcript,
        staged: &ports::storage::StagedArtifact,
        workspace_key: &domain::outbox::WorkspaceKey,
    ) -> Result<(), ApplicationError> {
        let mut revision_retries = 0;
        loop {
            let mut current_project = match self.project_repo.get(project_id).await {
                Ok(Some(project)) => project,
                Ok(None) => {
                    let error = ApplicationError::ProjectNotFound(project_id.clone());
                    return Err(self
                        .cleanup_coordinator
                        .handle_all_failure(&staged.staging_key, workspace_key, error)
                        .await);
                }
                Err(error) => {
                    return Err(self
                        .cleanup_coordinator
                        .handle_all_failure(&staged.staging_key, workspace_key, error.into())
                        .await);
                }
            };

            let invalid = if current_project.status() != &domain::project::ProjectStatus::Processing
            {
                Some("Project status is not Processing")
            } else if current_project.active_job_id() != Some(job_id) {
                Some("Active job ID mismatch")
            } else if current_project.source() != Some(source) {
                Some("Project source changed")
            } else if current_project.transcript().is_some() {
                Some("Project transcript was already changed")
            } else {
                None
            };
            if let Some(message) = invalid {
                return Err(self
                    .cleanup_coordinator
                    .handle_all_failure(
                        &staged.staging_key,
                        workspace_key,
                        ApplicationError::InvalidOperation {
                            message: message.into(),
                        },
                    )
                    .await);
            }

            let expected_project_updated_at = current_project.updated_at();
            let expected_status = current_project.status().clone();
            let expected_active_job_id = current_project.active_job_id().cloned();
            if let Err(error) = current_project.set_transcript(transcript.clone()) {
                return Err(self
                    .cleanup_coordinator
                    .handle_all_failure(&staged.staging_key, workspace_key, error.into())
                    .await);
            }

            let commit = self
                .storage_uow
                .commit_transcript_import(CommitTranscriptImport {
                    project: current_project,
                    artifact: staged.artifact.clone(),
                    staging_key: staged.staging_key.clone(),
                    final_key: staged.final_key.clone(),
                    temp_workspace_key: Some(workspace_key.clone()),
                    expected_project_updated_at,
                    expected_status,
                    expected_active_job_id,
                })
                .await;
            match commit {
                Ok(()) => return Ok(()),
                Err(ports::error::PortError::Conflict { .. }) if revision_retries < 2 => {
                    revision_retries += 1;
                }
                Err(error) => {
                    return Err(self
                        .cleanup_coordinator
                        .handle_all_failure(&staged.staging_key, workspace_key, error.into())
                        .await);
                }
            }
        }
    }
}
