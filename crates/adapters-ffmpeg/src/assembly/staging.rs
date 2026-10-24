use super::{
    config::AudioAssemblyLimits,
    control::WorkControl,
    error,
    pcm::{self, PcmStats},
};
use domain::speech::timeline::{AudioTimeline, TimelineSpan};
use ports::{PortError, audio_assembly::AudioAssemblyRequest};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
#[cfg(test)]
mod tests;

pub(super) struct PreparedAssembly {
    pub directory: TempDir,
    pub timeline: AudioTimeline,
    pub pcm_sha256: String,
    pub stats: PcmStats,
    pub raw_source: Option<ports::speech_audio::SpeechAudioLayout>,
}

pub(super) fn release(owned_root: &Path, directory: &Path) -> Result<(), PortError> {
    if !directory.is_absolute()
        || directory.parent() != Some(owned_root)
        || !directory
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("audio-assembly-"))
    {
        return Err(error::invalid(
            "Assembly cleanup target is outside its owned staging root",
        ));
    }
    let metadata = match fs::symlink_metadata(directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(super::error::io(error)),
    };
    let canonical = fs::canonicalize(directory).map_err(error::io)?;
    if !directory.is_absolute()
        || !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || canonical.parent() != Some(owned_root)
        || !canonical
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("audio-assembly-"))
    {
        return Err(error::invalid(
            "Assembly cleanup target is not a generated child of its owned staging root",
        ));
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&canonical).map_err(error::io)? {
        let entry = entry.map_err(error::io)?;
        if !entry.file_type().map_err(error::io)?.is_file() || files.len() >= 32772 {
            return Err(error::invalid(
                "Assembly staging cleanup requires a bounded flat directory of generated files",
            ));
        }
        files.push(entry.path());
    }
    for file in files {
        fs::remove_file(file).map_err(|error| PortError::CleanupFailed {
            resource: "audio_assembly_staging".into(),
            message: error.to_string(),
        })?;
    }
    fs::remove_dir(canonical).map_err(|error| PortError::CleanupFailed {
        resource: "audio_assembly_staging".into(),
        message: error.to_string(),
    })
}

pub(super) fn prepare(
    request: &AudioAssemblyRequest,
    timeline: AudioTimeline,
    input_root: &Path,
    stage_root: &Path,
    limits: &AudioAssemblyLimits,
    control: &WorkControl,
) -> Result<PreparedAssembly, PortError> {
    control.check()?;
    let directory = tempfile::Builder::new()
        .prefix("audio-assembly-")
        .tempdir_in(stage_root)
        .map_err(error::io)?;
    let mut copied = Vec::<PathBuf>::with_capacity(request.inputs.len());
    let mut total_bytes = 0_u64;
    for (index, (input, facts)) in request
        .inputs
        .iter()
        .zip(&request.timeline.utterances)
        .enumerate()
    {
        control.check()?;
        let metadata = fs::symlink_metadata(&input.path).map_err(error::io)?;
        let source = fs::canonicalize(&input.path).map_err(error::io)?;
        if !input.path.is_absolute()
            || !metadata.is_file()
            || metadata.file_type().is_symlink()
            || !source.starts_with(input_root)
        {
            return Err(error::invalid(
                "Utterance audio is outside the owned input root or is a link",
            ));
        }
        let target = directory.path().join(format!("speech-{index:05}.wav"));
        let remaining = limits.max_input_bytes.saturating_sub(total_bytes);
        total_bytes += pcm::copy_verified(
            &source,
            &target,
            facts.audio_sha256.as_str(),
            remaining,
            control,
        )?;
        let properties = common::wav::inspect_pcm16_wav(&target).map_err(error::wave)?;
        if properties.sample_rate != 48000
            || properties.channels != timeline.channels
            || properties.frames != facts.frame_count
            || properties.sample_rate != facts.sample_rate
        {
            return Err(error::invalid(
                "Measured utterance properties differ from immutable facts",
            ));
        }
        copied.push(target);
    }
    let mut expected = Sha256::new();
    let mut stats = PcmStats::default();
    let mut speech_index = 0;
    let mut list = String::from("ffconcat version 1.0\n");
    for (index, span) in timeline.spans.iter().enumerate() {
        control.check()?;
        let filename = match span {
            TimelineSpan::Speech {
                utterance_id,
                audio_sha256,
                frame_count,
                ..
            } => {
                let input = request
                    .inputs
                    .get(speech_index)
                    .ok_or_else(|| error::invalid("Missing utterance audio"))?;
                let facts = &request.timeline.utterances[speech_index];
                if &input.utterance_id != utterance_id
                    || &facts.audio_sha256 != audio_sha256
                    || *frame_count != facts.frame_count
                {
                    return Err(error::invalid(
                        "Timeline utterance lineage differs from the prepared audio",
                    ));
                }
                pcm::append_pcm(&copied[speech_index], &mut expected, &mut stats, control)?;
                let filename = format!("speech-{speech_index:05}.wav");
                speech_index += 1;
                filename
            }
            TimelineSpan::Silence { frame_count, .. } => {
                let filename = format!("silence-{index:05}.wav");
                pcm::write_silence(
                    &directory.path().join(&filename),
                    *frame_count,
                    timeline.channels,
                    &mut expected,
                    &mut stats,
                    control,
                )?;
                filename
            }
        };
        list.push_str(&format!("file '{filename}'\n"));
    }
    if speech_index != request.inputs.len()
        || stats.samples != timeline.frame_count * u64::from(timeline.channels)
    {
        return Err(error::invalid(
            "Prepared PCM does not cover the complete timeline",
        ));
    }
    let mut script = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.path().join("timeline.ffconcat"))
        .map_err(error::io)?;
    script.write_all(list.as_bytes()).map_err(error::io)?;
    script.sync_all().map_err(error::io)?;
    Ok(PreparedAssembly {
        directory,
        timeline,
        pcm_sha256: format!("{:x}", expected.finalize()),
        stats,
        raw_source: None,
    })
}
