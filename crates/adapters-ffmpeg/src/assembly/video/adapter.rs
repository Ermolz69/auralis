use super::super::{
    control::{CancelOnDrop, WorkControl},
    error, pcm,
};
use super::{input, probe};
use async_trait::async_trait;
use domain::speech::Sha256Digest;
use ports::{
    PortError,
    cancellation::CancellationToken,
    video_export::{
        StagedVideoExport, VideoExportPort, VideoExportRequest, VideoInspection, VideoSourceInput,
    },
};
use std::{fs, future::Future, path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct FfmpegVideoExporter {
    pub(super) ffmpeg: PathBuf,
    pub(super) ffmpeg_pin: Sha256Digest,
    pub(super) ffprobe: PathBuf,
    pub(super) ffprobe_pin: Sha256Digest,
    pub(super) input_root: PathBuf,
    pub(super) staging_root: PathBuf,
}

impl FfmpegVideoExporter {
    pub fn new(
        ffmpeg: PathBuf,
        ffmpeg_pin: Sha256Digest,
        ffprobe: PathBuf,
        ffprobe_pin: Sha256Digest,
        input_root: PathBuf,
        staging_root: PathBuf,
    ) -> Result<Self, PortError> {
        if !ffmpeg.is_absolute() || !ffprobe.is_absolute() {
            return Err(error::invalid("Media executables require absolute paths"));
        }
        Ok(Self {
            ffmpeg,
            ffmpeg_pin,
            ffprobe,
            ffprobe_pin,
            input_root: fs::canonicalize(input_root).map_err(error::io)?,
            staging_root: fs::canonicalize(staging_root).map_err(error::io)?,
        })
    }

    pub(super) async fn inspect_inner(
        &self,
        source: VideoSourceInput,
        control: WorkControl,
    ) -> Result<VideoInspection, PortError> {
        let owner = self.clone();
        let work = control.clone();
        let source_copy = source.clone();
        let digest = tokio::task::spawn_blocking(move || {
            for (file, pin) in [
                (&owner.ffmpeg, &owner.ffmpeg_pin),
                (&owner.ffprobe, &owner.ffprobe_pin),
            ] {
                if pcm::hash_file(file, &work)? != pin.as_str() {
                    return Err(error::invalid("Pinned video tool changed"));
                }
            }
            input::inspect(&owner.input_root, &source_copy, &work)
        })
        .await
        .map_err(|error| super::super::error::external(error.to_string()))??;
        let media = probe::probe(&self.ffprobe, &source.path, control.cancel.clone()).await?;
        control.check()?;
        let duration_ms = super::policy::source_duration(&media)?;
        if !media.has_video
            || media.duration_ms == 0
            || media.duration_ms > 28800000
            || media.width.is_none_or(|width| !(2..=3840).contains(&width))
            || media
                .height
                .is_none_or(|height| !(2..=2160).contains(&height))
        {
            return Err(error::invalid("Unsupported video dimensions or duration"));
        }
        Ok(VideoInspection {
            sha256: Sha256Digest::parse(digest).map_err(error::invalid)?,
            size_bytes: source.size_bytes,
            duration_ms,
            width: media.width.unwrap_or(0),
            height: media.height.unwrap_or(0),
            video_codec: media
                .video_codec
                .ok_or_else(|| error::invalid("Source video codec is missing"))?,
            ffprobe_sha256: self.ffprobe_pin.clone(),
        })
    }
}

pub(super) async fn admitted<T>(
    operation: impl Future<Output = Result<T, PortError>>,
) -> Result<T, PortError> {
    let lease = tokio::task::spawn_blocking(
        common::model_admission::ModelAdmissionLease::acquire_model_free,
    )
    .await
    .map_err(|error| super::super::error::external(error.to_string()))?
    .map_err(|error| PortError::Busy {
        message: error.to_string(),
    })?;
    let outcome = operation.await;
    if matches!(&outcome, Err(PortError::CleanupFailed {resource, ..}) if resource == "ffmpeg_process")
    {
        std::mem::forget(lease);
        return outcome;
    }
    tokio::task::spawn_blocking(move || lease.release())
        .await
        .map_err(|error| super::super::error::external(error.to_string()))?
        .map_err(|error| PortError::CleanupFailed {
            resource: "audio_machine_admission".into(),
            message: error.to_string(),
        })?;
    outcome
}

#[async_trait]
impl VideoExportPort for FfmpegVideoExporter {
    async fn inspect(
        &self,
        source: VideoSourceInput,
        cancel: CancellationToken,
    ) -> Result<VideoInspection, PortError> {
        let cancel_on_drop = CancelOnDrop(cancel.clone());
        let control = WorkControl::new(cancel, Duration::from_secs(180))?;
        let owner = self.clone();
        let result =
            tokio::spawn(async move { admitted(owner.inspect_inner(source, control)).await })
                .await
                .map_err(|error| super::super::error::external(error.to_string()))?;
        drop(cancel_on_drop);
        result
    }
    async fn export(
        &self,
        request: VideoExportRequest,
        cancel: CancellationToken,
    ) -> Result<StagedVideoExport, PortError> {
        let cancel_on_drop = CancelOnDrop(cancel.clone());
        let control = WorkControl::new(cancel, Duration::from_secs(1800))?;
        let owner = self.clone();
        let result = tokio::spawn(async move { admitted(owner.render(request, control)).await })
            .await
            .map_err(|error| super::super::error::external(error.to_string()))?;
        drop(cancel_on_drop);
        result
    }
}
