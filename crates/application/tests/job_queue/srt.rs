use super::support::*;
use application::usecases::pipeline::start_mock::StartMockPipelineRequest;
use domain::{
    job::JobStatus,
    media::{Artifact, ArtifactKind, ArtifactLocation, ArtifactState, MediaSource, SubtitleTrack},
};
use ports::{
    PortError,
    repository::ProjectRepository,
    source::{DownloadSubtitleRequest, SubtitleSourcePort},
};
use std::sync::{Arc, Mutex};

const SOURCE: &str = "1\r\n00:00:00,100 --> 00:00:01,500\r\n你好。\r\n世界。\r\n\r\n2\r\n00:00:01,500 --> 00:00:02,800\r\n再见。\r\n";
#[derive(Clone)]
struct SrtSource {
    valid: bool,
    requested: Arc<Mutex<Vec<SubtitleTrack>>>,
}
fn track() -> SubtitleTrack {
    SubtitleTrack {
        id: "zh-manual-srt".into(),
        language: "zh".into(),
        label: Some("Chinese".into()),
        format: Some("srt".into()),
        is_auto_generated: false,
    }
}
#[async_trait::async_trait]
impl SubtitleSourcePort for SrtSource {
    async fn list_subtitles(&self, _: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError> {
        Ok(["json3", "vtt", "srt"]
            .into_iter()
            .map(|format| SubtitleTrack {
                id: format!("zh-manual-{format}"),
                format: Some(format.into()),
                ..track()
            })
            .collect())
    }
    async fn download_subtitle(
        &self,
        request: DownloadSubtitleRequest,
    ) -> Result<Artifact, PortError> {
        self.requested.lock().unwrap().push(request.track);
        let path = request.target_directory.join("chosen.srt");
        tokio::fs::write(
            &path,
            if self.valid {
                SOURCE
            } else {
                "1\ninvalid timing\n坏字幕\n"
            },
        )
        .await
        .map_err(|e| PortError::Io {
            message: e.to_string(),
        })?;
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::OriginalSubtitle,
            location: ArtifactLocation::LocalPath(path.to_string_lossy().into_owned()),
            size_bytes: None,
            state: ArtifactState::PendingFinalize,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: None,
        })
    }
}

#[tokio::test]
async fn queued_selected_srt_retains_format_bytes_and_cues_and_rejects_bad_grammar() {
    for valid in [true, false] {
        let harness = Harness::new(jobs::queue::JobQueueConfig::default()).await;
        let requested = Arc::new(Mutex::new(Vec::new()));
        let source = SrtSource {
            valid,
            requested: requested.clone(),
        };
        let pipeline = harness
            .pipeline(source.clone(), harness.workspace())
            .with_subtitle_inspector(Arc::new(adapters_translate::FormatInspector));
        harness
            .manager
            .start_dispatcher(pipeline.queued_executor())
            .await
            .unwrap();
        let project = harness.project().await;
        let listed = application::usecases::transcript::list_youtube_tracks::ListYoutubeSubtitleTracksUseCase::new(harness.projects.clone(), source)
            .execute(application::usecases::transcript::list_youtube_tracks::ListYoutubeSubtitleTracksRequest { project_id: project.id().clone() }).await.unwrap();
        assert_eq!(
            listed
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            ["zh-manual-srt", "zh-manual-vtt"]
        );
        let admitted = pipeline
            .execute(StartMockPipelineRequest {
                project_id: project.id().clone(),
                selected_subtitle_track: Some(track()),
            })
            .await
            .unwrap();
        harness
            .wait_status(
                &admitted.job.id,
                if valid {
                    JobStatus::Completed
                } else {
                    JobStatus::Failed
                },
            )
            .await;
        assert_eq!(*requested.lock().unwrap(), vec![track()]);
        let saved = harness.projects.get(project.id()).await.unwrap().unwrap();
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT location_value,size_bytes FROM artifacts WHERE kind='OriginalSubtitle'",
        )
        .fetch_all(&harness.pool)
        .await
        .unwrap();
        if valid {
            let transcript = saved.transcript().unwrap();
            assert_eq!(transcript.language, "zh");
            assert_eq!(transcript.segments.len(), 2);
            assert_eq!(transcript.segments[0].source_text, "你好。\n世界。");
            assert_eq!(transcript.segments[0].start_ms, 100);
            assert_eq!(transcript.segments[1].end_ms, 2800);
            assert_eq!(rows.len(), 1);
            assert!(rows[0].0.ends_with(".srt"));
            assert_eq!(rows[0].1, SOURCE.len() as i64);
        } else {
            assert!(saved.transcript().is_none());
            assert!(rows.is_empty());
        }
        harness.shutdown().await;
    }
}
