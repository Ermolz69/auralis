use super::super::error;
use domain::media::MediaMetadata;
use ports::PortError;

pub(super) fn source_duration(media: &MediaMetadata) -> Result<u64, PortError> {
    let videos: Vec<_> = media
        .streams
        .iter()
        .filter(|stream| stream.codec_type == domain::media::CodecType::Video)
        .collect();
    let duration = videos
        .first()
        .and_then(|stream| stream.duration_ms)
        .filter(|duration| *duration > 0 && *duration <= 28800000)
        .ok_or_else(|| error::invalid("Source video stream has no supported exact duration"))?;
    if videos.len() != 1 || duration.abs_diff(media.duration_ms) > 200 {
        return Err(error::invalid(
            "Source video streams or container timing require explicit review",
        ));
    }
    Ok(duration)
}

pub(super) fn duration(frames: u64, video_ms: u64) -> Result<(), PortError> {
    if video_ms == 0 || video_ms > 28800000 || frames == 0 || frames > video_ms.saturating_mul(48) {
        return Err(error::invalid(
            "Speech exceeds source video duration; review timing without trimming speech",
        ));
    }
    Ok(())
}

pub(super) fn output(
    media: &MediaMetadata,
    duration_ms: u64,
    channels: u16,
) -> Result<(), PortError> {
    if media.streams.len() != 2
        || media.streams.iter().any(|stream| {
            stream
                .duration_ms
                .is_none_or(|duration| duration.abs_diff(duration_ms) > 200)
        })
        || !media.has_video
        || !media.has_audio
        || media.video_codec.as_deref() != Some("h264")
        || media.audio_codec.as_deref() != Some("aac")
        || media.sample_rate != Some(48000)
        || media.audio_channels.map(u16::from) != Some(channels)
        || media.duration_ms.abs_diff(duration_ms) > 200
        || media.audio_tracks.len() != 1
        || media.audio_tracks[0].language.as_deref() != Some("rus")
    {
        return Err(error::invalid(
            "Exported MP4 streams or duration differ from the reviewed policy",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn speech_is_never_silently_trimmed_and_video_budget_is_finite() {
        assert!(super::duration(48000, 1000).is_ok());
        assert!(super::duration(48001, 1000).is_err());
        assert!(super::duration(0, 1000).is_err());
        assert!(super::duration(1, 28800001).is_err());
    }
    #[test]
    fn container_duration_cannot_hide_a_missing_video_tail() {
        let bytes = serde_json::json!({"format":{"duration":"1.000"},"streams":[
            {"index":0,"codec_type":"video","codec_name":"h264","width":320,"height":180,"duration":"1.000"},
            {"index":1,"codec_type":"audio","codec_name":"aac","channels":1,"sample_rate":"48000","duration":"1.000","tags":{"language":"rus"}}
        ]});
        let output = serde_json::from_value(bytes).expect("fixture probe DTO");
        let mut media =
            crate::ffprobe::parser::parse_ffprobe_output(&output).expect("fixture metadata");
        assert!(super::output(&media, 1000, 1).is_ok());
        media.streams[0].duration_ms = Some(500);
        assert!(super::output(&media, 1000, 1).is_err());
        assert!(super::source_duration(&media).is_err());
    }
}
