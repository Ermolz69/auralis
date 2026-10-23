use super::{
    AudioAssemblyLimits,
    control::WorkControl,
    error,
    input::{self, AssemblyInput},
    pcm::{self, PcmStats},
    process,
};
use async_trait::async_trait;
use domain::speech::Sha256Digest;
use ports::{
    PortError,
    audio_assembly::{AudioAssemblyPort, AudioAssemblyRequest, StagedAudioAssembly},
    cancellation::CancellationToken,
    speech_audio::{SpeechAudioAssemblyPort, SpeechAudioAssemblyRequest},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::Instant,
};
use tempfile::TempDir;
use tokio::process::Command;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct FfmpegAudioAssembler {
    executable: PathBuf,
    executable_sha256: Sha256Digest,
    input_root: PathBuf,
    staging_root: PathBuf,
    limits: AudioAssemblyLimits,
}

impl FfmpegAudioAssembler {
    pub fn new(
        executable: PathBuf,
        executable_sha256: Sha256Digest,
        input_root: PathBuf,
        staging_root: PathBuf,
        limits: AudioAssemblyLimits,
    ) -> Result<Self, PortError> {
        limits.validate()?;
        if !executable.is_absolute() || !input_root.is_absolute() || !staging_root.is_absolute() {
            return Err(error::invalid(
                "Audio assembly paths must be explicit absolute paths",
            ));
        }
        Ok(Self {
            executable,
            executable_sha256,
            input_root: fs::canonicalize(input_root).map_err(error::io)?,
            staging_root: fs::canonicalize(staging_root).map_err(error::io)?,
            limits,
        })
    }

    async fn admitted(
        &self,
        request: AssemblyInput,
        control: WorkControl,
    ) -> Result<(TempDir, StagedAudioAssembly), PortError> {
        let owner = self.clone();
        let work = control.clone();
        let prepared = tokio::task::spawn_blocking(move || {
            if pcm::hash_file(&owner.executable, &work)? != owner.executable_sha256.as_str() {
                return Err(error::invalid("Managed FFmpeg executable hash differs"));
            }
            input::prepare(
                request,
                &owner.input_root,
                &owner.staging_root,
                &owner.limits,
                &work,
            )
        })
        .await
        .map_err(|error| super::error::external(error.to_string()))??;
        control.check()?;
        let mut command = Command::new(&self.executable);
        command.current_dir(prepared.directory.path()).args([
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-nostats",
            "-n",
            "-f",
            "concat",
            "-safe",
            "1",
            "-protocol_whitelist",
            "file",
            "-i",
            "timeline.ffconcat",
            "-map",
            "0:a:0",
            "-c:a",
            "copy",
            "-map_metadata",
            "-1",
            "-fflags",
            "+bitexact",
            "-f",
            "wav",
            "speech.wav",
        ]);
        let deadline = self
            .limits
            .process_timeout
            .min(control.deadline.saturating_duration_since(Instant::now()));
        process::run(
            command,
            control.cancel.clone(),
            deadline,
            self.limits.max_stderr_bytes,
        )
        .await?;
        let tool_hash = self.executable_sha256.clone();
        tokio::task::spawn_blocking(move || {
            control.check()?;
            let wav_path = prepared.directory.path().join("speech.wav");
            let mut digest = Sha256::new();
            let mut stats = PcmStats::default();
            let actual = pcm::append_pcm(&wav_path, &mut digest, &mut stats, &control)?;
            let pcm_hash = format!("{:x}", digest.finalize());
            if actual.sample_rate != 48000 || actual.channels != prepared.timeline.channels
                || actual.frames != prepared.timeline.frame_count || pcm_hash != prepared.pcm_sha256 || stats != prepared.stats
            { return Err(error::invalid("FFmpeg PCM output differs from exact timeline coverage or samples")); }
            let wav_hash = pcm::hash_file(&wav_path, &control)?;
            let manifest_path = prepared.directory.path().join("assembly.json");
            let manifest = serde_json::json!({
                "version": 1, "assembly_policy": "pcm16_48k_exact_concat_v1",
                "ffmpeg_sha256": tool_hash, "timeline": prepared.timeline,
                "output_file": "speech.wav", "output_sha256": wav_hash, "pcm_sha256": pcm_hash,
                "peak_sample_abs": stats.peak, "full_scale_samples": stats.full_scale, "sample_count": stats.samples,
                "raw_source": prepared.raw_source,
            });
            let data = serde_json::to_vec_pretty(&manifest).map_err(|error| super::error::invalid(error.to_string()))?;
            let mut file = OpenOptions::new().write(true).create_new(true).open(&manifest_path).map_err(error::io)?;
            file.write_all(&data).map_err(error::io)?;
            file.sync_all().map_err(error::io)?;
            OpenOptions::new().write(true).open(&wav_path).map_err(error::io)?.sync_all().map_err(error::io)?;
            control.check()?;
            let result = StagedAudioAssembly {
                staging_root: prepared.directory.path().into(), wav_path, manifest_path,
                wav_sha256: Sha256Digest::parse(wav_hash).map_err(error::invalid)?,
                manifest_sha256: Sha256Digest::parse(format!("{:x}", Sha256::digest(&data))).map_err(error::invalid)?,
                pcm_sha256: Sha256Digest::parse(pcm_hash).map_err(error::invalid)?,
                sample_rate: actual.sample_rate, channels: actual.channels, frames: actual.frames,
                peak_sample_abs: stats.peak, full_scale_samples: stats.full_scale,
                admission_guardian_pid: 0,
            };
            Ok((prepared.directory, result))
        }).await.map_err(|error| super::error::external(error.to_string()))?
    }
}

impl FfmpegAudioAssembler {
    async fn assemble_owned(
        &self,
        request: AssemblyInput,
        cancellation: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError> {
        let control = WorkControl::new(cancellation, self.limits.total_timeout)?;
        control.check()?;
        let admission = tokio::task::spawn_blocking(
            common::model_admission::ModelAdmissionLease::acquire_model_free,
        )
        .await
        .map_err(|error| super::error::external(error.to_string()))?
        .map_err(|error| PortError::Busy {
            message: error.to_string(),
        })?;
        let guardian_pid = admission.guardian_pid();
        let outcome = self.admitted(request, control.clone()).await;
        if matches!(&outcome, Err(PortError::CleanupFailed { resource, .. }) if resource == "ffmpeg_process")
        {
            std::mem::forget(admission);
            return outcome.map(|(_, result)| result);
        }
        tokio::task::spawn_blocking(move || admission.release())
            .await
            .map_err(|error| super::error::external(error.to_string()))?
            .map_err(|error| PortError::CleanupFailed {
                resource: "audio_machine_admission".into(),
                message: format!("guardian {guardian_pid}: {error}"),
            })?;
        control.check()?;
        let (directory, mut result) = outcome?;
        result.admission_guardian_pid = guardian_pid;
        let _ = directory.keep();
        Ok(result)
    }
}

#[async_trait]
impl AudioAssemblyPort for FfmpegAudioAssembler {
    async fn assemble(
        &self,
        request: AudioAssemblyRequest,
        cancellation: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError> {
        self.dispatch(AssemblyInput::Files(request), cancellation)
            .await
    }
}

#[async_trait]
impl SpeechAudioAssemblyPort for FfmpegAudioAssembler {
    async fn assemble_speech(
        &self,
        request: SpeechAudioAssemblyRequest,
        cancellation: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError> {
        self.dispatch(AssemblyInput::Speech(request), cancellation)
            .await
    }
}

impl FfmpegAudioAssembler {
    async fn dispatch(
        &self,
        request: AssemblyInput,
        cancellation: CancellationToken,
    ) -> Result<StagedAudioAssembly, PortError> {
        let owner = self.clone();
        let cancellation = cancellation.child_token();
        let cancel_on_drop = super::control::CancelOnDrop(cancellation.clone());
        let operation =
            tokio::spawn(async move { owner.assemble_owned(request, cancellation).await });
        let result = operation
            .await
            .map_err(|error| super::error::external(error.to_string()))?;
        drop(cancel_on_drop);
        result
    }
}
