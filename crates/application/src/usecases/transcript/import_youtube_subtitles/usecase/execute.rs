use super::*;

impl ImportYoutubeSubtitlesUseCase {
    pub async fn execute(
        &self,
        request: ImportYoutubeSubtitlesRequest,
    ) -> Result<ImportYoutubeSubtitlesResponse, ApplicationError> {
        let project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        let source = project
            .source()
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Project has no source".to_string(),
            })?;

        let best_track = if let Some(track) = request.selected_track {
            if track.format.as_deref() != Some("vtt") || track.language.trim().is_empty() {
                return Err(ApplicationError::InvalidOperation {
                    message: "Invalid selected subtitle track".to_string(),
                });
            }
            track
        } else {
            let subtitles = self
                .subtitle_source
                .list_subtitles_cancellable(source, &request.cancellation_token)
                .await?;
            if subtitles.is_empty() {
                return Err(ApplicationError::InvalidOperation {
                    message: "No subtitles found".to_string(),
                });
            }
            select_best_subtitle_track(
                &subtitles,
                &request.preferred_languages,
                request.allow_auto_generated,
            )?
        };

        let alloc = self
            .workspace_port
            .create_allocation(&request.project_id, "subtitles")
            .await?;

        let download_request = DownloadSubtitleRequest {
            source: source.clone(),
            track: best_track.clone(),
            target_directory: alloc.absolute_path.clone(),
        };

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_workspace_failure(
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        let artifact = match self
            .subtitle_source
            .download_subtitle_cancellable(download_request, &request.cancellation_token)
            .await
        {
            Ok(a) => a,
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, e.into())
                    .await);
            }
        };

        let vtt_path = match &artifact.location {
            domain::media::ArtifactLocation::LocalPath(p) => std::path::PathBuf::from(p),
            _ => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(
                        &alloc.workspace_key,
                        ApplicationError::InvalidOperation {
                            message: "Invalid subtitle artifact location".to_string(),
                        },
                    )
                    .await);
            }
        };

        let filename = match vtt_path.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(
                        &alloc.workspace_key,
                        ApplicationError::InvalidOperation {
                            message: "Invalid subtitle filename".to_string(),
                        },
                    )
                    .await);
            }
        };

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_workspace_failure(
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        let vtt_content = match self
            .workspace_port
            .read_workspace_file_to_string(&alloc.workspace_key, &filename, self.max_subtitle_bytes)
            .await
        {
            Ok(content) => content,
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, e.into())
                    .await);
            }
        };

        let parsed = if let Some(cpu) = &self.cpu_executor {
            let language = best_track.language.clone();
            let (send, receive) = tokio::sync::oneshot::channel();
            match cpu
                .run(Box::new(move || {
                    let _ = send.send(parse_vtt(&vtt_content, &language));
                }))
                .await
            {
                Ok(()) => receive
                    .await
                    .map_err(|_| ApplicationError::InvalidOperation {
                        message: "Subtitle parser did not return a result".into(),
                    })
                    .and_then(|result| result),
                Err(error) => Err(error.into()),
            }
        } else {
            parse_vtt(&vtt_content, &best_track.language)
        };
        let transcript = match parsed {
            Ok(t) => t,
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, e)
                    .await);
            }
        };

        if let Err(e) = transcript.validate() {
            return Err(self
                .cleanup_coordinator
                .handle_workspace_failure(&alloc.workspace_key, e.into())
                .await);
        }

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_workspace_failure(
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        let staged = match self
            .artifact_store
            .stage_owned_workspace_file(
                &request.project_id,
                domain::media::ArtifactKind::OriginalSubtitle,
                self.workspace_port.as_ref(),
                &alloc.workspace_key,
                &filename,
                Some("subtitles.vtt"),
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, e.into())
                    .await);
            }
        };

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_all_failure(
                    &staged.staging_key,
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        self.commit_staged_transcript(
            &request.project_id,
            &request.job_id,
            source,
            &transcript,
            &staged,
            &alloc.workspace_key,
        )
        .await?;

        if self.cpu_executor.is_some() {
            // Staging and its finalization outbox now own the committed bytes. The
            // queued worker still joins its temporary workspace cleanup before exit.
            let cleanup = self
                .cleanup_coordinator
                .cleanup_workspace(&alloc.workspace_key)
                .await;
            if !cleanup.is_empty() {
                return Err(cleanup.into_error(ApplicationError::InvalidOperation {
                    message: "Committed import workspace cleanup needs attention".into(),
                }));
            }
        }

        Ok(ImportYoutubeSubtitlesResponse { transcript })
    }
}
