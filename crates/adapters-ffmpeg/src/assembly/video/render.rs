use super::super::{control::WorkControl, error, pcm, process};
use super::{adapter::FfmpegVideoExporter, input, policy, probe};
use domain::speech::Sha256Digest;
use ports::{
    PortError,
    video_export::{StagedVideoExport, VideoExportRequest},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    time::Duration,
};
use tokio::process::Command;

impl FfmpegVideoExporter {
    pub(super) async fn render(
        &self,
        request: VideoExportRequest,
        control: WorkControl,
    ) -> Result<StagedVideoExport, PortError> {
        if request.policy_id != ports::video_export::REPLACE_AUDIO_POLICY {
            return Err(error::invalid("Video policy changed; review a new plan"));
        }
        let observed = self
            .inspect_inner(request.source.clone(), control.clone())
            .await?;
        if observed != request.reviewed {
            return Err(error::invalid("Video changed after timing review"));
        }
        policy::duration(request.frames, observed.duration_ms)?;
        if observed
            .duration_ms
            .saturating_mul(48)
            .saturating_mul(u64::from(request.channels) * 2)
            > (1_u64 << 32) - 128
        {
            return Err(error::invalid(
                "Video verification exceeds the PCM byte budget",
            ));
        }
        let directory = fs::canonicalize(&request.output_directory).map_err(error::io)?;
        if directory.parent() != Some(self.staging_root.as_path())
            || !directory
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("audio-assembly-"))
        {
            return Err(error::invalid(
                "Video output must belong to the current assembly staging directory",
            ));
        }
        let video = directory.join("source-video.snapshot");
        let wav = directory.join("speech.snapshot.wav");
        let output = directory.join("russian.mp4");
        let decoded = directory.join("decoded-video.wav");
        let manifest = directory.join("video-export.json");
        for target in [&video, &wav, &output, &decoded, &manifest] {
            if target.exists() {
                return Err(error::invalid("Video staging output already exists"));
            }
        }
        let owned_request = request;
        let work = control.clone();
        let source_root = self.input_root.clone();
        let wav_copy = wav.clone();
        let video_copy = video.clone();
        let (request, source_stats) = tokio::task::spawn_blocking(move || {
            input::inspect(&source_root, &owned_request.source, &work)?;
            pcm::copy_verified(
                &owned_request.source.path,
                &video_copy,
                owned_request.reviewed.sha256.as_str(),
                1_u64 << 32,
                &work,
            )?;
            input::managed_file(&directory, &owned_request.wav_path, 1_u64 << 32)?;
            pcm::copy_verified(
                &owned_request.wav_path,
                &wav_copy,
                owned_request.wav_sha256.as_str(),
                1_u64 << 32,
                &work,
            )?;
            let mut stats = pcm::PcmStats::default();
            let properties = pcm::append_pcm(&wav_copy, &mut Sha256::new(), &mut stats, &work)?;
            if properties.sample_rate != 48000
                || properties.channels != owned_request.channels
                || properties.frames != owned_request.frames
                || stats.full_scale != 0
            {
                return Err(error::invalid(
                    "Reviewed WAV measurements or clipping differ",
                ));
            }
            Ok((owned_request, stats))
        })
        .await
        .map_err(|error| super::super::error::external(error.to_string()))??;
        let mut encode = Command::new(&self.ffmpeg);
        encode
            .args([
                "-nostdin",
                "-v",
                "error",
                "-n",
                "-protocol_whitelist",
                "file,pipe",
                "-i",
            ])
            .arg(&video)
            .arg("-i")
            .arg(&wav)
            .args([
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "20",
                "-vf",
                "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-af",
                "apad",
                "-t",
            ])
            .arg(format!(
                "{}.{:03}",
                observed.duration_ms / 1000,
                observed.duration_ms % 1000
            ))
            .args([
                "-threads",
                "4",
                "-map_metadata",
                "-1",
                "-metadata:s:a:0",
                "language=rus",
                "-movflags",
                "+faststart",
                "-sn",
                "-dn",
                "-fs",
                "4294967296",
            ])
            .arg(&output);
        process::run(
            encode,
            control.cancel.clone(),
            Duration::from_secs(1200),
            65536,
        )
        .await?;
        let media = probe::probe(&self.ffprobe, &output, control.cancel.clone()).await?;
        policy::output(&media, observed.duration_ms, request.channels)?;
        let mut decode = Command::new(&self.ffmpeg);
        decode
            .args([
                "-nostdin",
                "-v",
                "error",
                "-n",
                "-protocol_whitelist",
                "file,pipe",
                "-i",
            ])
            .arg(&output)
            .args(["-map", "0:a:0", "-vn", "-c:a", "pcm_s16le"])
            .arg(&decoded);
        process::run(
            decode,
            control.cancel.clone(),
            Duration::from_secs(300),
            65536,
        )
        .await?;
        let work = control.clone();
        tokio::task::spawn_blocking(move || {
            let mut stats = pcm::PcmStats::default();
            let decoded_properties = pcm::append_pcm(&decoded, &mut Sha256::new(), &mut stats, &work)?;
            if decoded_properties.sample_rate != 48000 || decoded_properties.channels != request.channels
                || decoded_properties.frames.abs_diff(observed.duration_ms * 48) > 2048 || stats.full_scale != 0 {
                return Err(error::invalid("Decoded MP4 audio duration, channels or clipping failed verification"));
            }
            let bytes = fs::metadata(&output).map_err(error::io)?.len();
            if bytes == 0 || bytes > 1_u64 << 32 { return Err(error::invalid("MP4 output byte budget exceeded")); }
            let digest = pcm::hash_file(&output, &work)?;
            let contents = serde_json::to_vec(&serde_json::json!({"version":1,"policy":"replace_original_audio_pad_silence_h264_aac_v1",
                "assembly_plan_sha256":request.assembly_plan_sha256,"source":observed,"wav_sha256":request.wav_sha256,
                "speech_frames":request.frames,"source_peak":source_stats.peak,"output_sha256":digest,"output_bytes":bytes,
                "decoded_frames":decoded_properties.frames,"decoded_peak":stats.peak,"decoded_full_scale_samples":stats.full_scale,"media":media}))
                .map_err(|error| super::super::error::invalid(error.to_string()))?;
            let mut file = OpenOptions::new().write(true).create_new(true).open(&manifest).map_err(error::io)?;
            file.write_all(&contents).map_err(error::io)?;
            file.sync_all().map_err(error::io)?;
            Ok(StagedVideoExport {video_path: output, manifest_path: manifest,
                video_sha256:Sha256Digest::parse(digest).map_err(error::invalid)?,
                manifest_sha256:Sha256Digest::parse(format!("{:x}",Sha256::digest(&contents))).map_err(error::invalid)?})
        }).await.map_err(|error| super::super::error::external(error.to_string()))?
    }
}
