use crate::ffprobe::dto::FfprobeOutput;
use crate::ffprobe::error::FfprobeError;
use domain::media::{
    AudioTrackMetadata, CodecType, MediaMetadata, MediaStream, VideoStreamMetadata,
};

pub fn parse_ffprobe_output(output: &FfprobeOutput) -> Result<MediaMetadata, FfprobeError> {
    let duration_ms = parse_duration(&output.format.duration).unwrap_or(0);
    let bitrate = output
        .format
        .bit_rate
        .as_ref()
        .and_then(|b| b.parse::<u64>().ok());
    let container = output.format.format_name.clone();
    let format_name = output.format.format_long_name.clone();

    let mut has_video = false;
    let mut has_audio = false;

    let mut streams = Vec::new();
    let mut video_metadata = None;
    let mut audio_tracks = Vec::new();

    // Collect first seen for simple legacy fields
    let mut first_width = None;
    let mut first_height = None;
    let mut first_fps = None;
    let mut first_vcodec = None;
    let mut first_acodec = None;
    let mut first_channels = None;
    let mut first_sample_rate = None;

    for stream in &output.streams {
        let codec_type = parse_codec_type(stream.codec_type.as_deref());
        let stream_duration = parse_duration(&stream.duration);

        let is_default = stream.disposition.get("default").copied().unwrap_or(0) == 1;
        let language = stream.tags.get("language").cloned();
        let title = stream.tags.get("title").cloned();

        let domain_stream = MediaStream {
            index: stream.index,
            codec_type: codec_type.clone(),
            codec_name: stream.codec_name.clone(),
            codec_long_name: stream.codec_long_name.clone(),
            language: language.clone(),
            duration_ms: stream_duration,
        };
        streams.push(domain_stream);

        match codec_type {
            CodecType::Video => {
                has_video = true;
                let fps = parse_fps(stream.r_frame_rate.as_deref());

                if first_width.is_none() {
                    first_width = stream.width;
                    first_height = stream.height;
                    first_fps = fps;
                    first_vcodec = stream.codec_name.clone();
                }

                if video_metadata.is_none() {
                    video_metadata = Some(VideoStreamMetadata {
                        stream_index: stream.index,
                        width: stream.width.unwrap_or(0),
                        height: stream.height.unwrap_or(0),
                        fps,
                        codec: stream.codec_name.clone(),
                        pixel_format: stream.pix_fmt.clone(),
                    });
                }
            }
            CodecType::Audio => {
                has_audio = true;
                let sample_rate = stream
                    .sample_rate
                    .as_ref()
                    .and_then(|s| s.parse::<u32>().ok());

                if first_acodec.is_none() {
                    first_acodec = stream.codec_name.clone();
                    first_channels = stream.channels;
                    first_sample_rate = sample_rate;
                }

                audio_tracks.push(AudioTrackMetadata {
                    stream_index: stream.index,
                    codec: stream.codec_name.clone(),
                    channels: stream.channels,
                    channel_layout: stream.channel_layout.clone(),
                    sample_rate,
                    language,
                    title,
                    is_default,
                });
            }
            _ => {}
        }
    }

    Ok(MediaMetadata {
        duration_ms,
        width: first_width,
        height: first_height,
        fps: first_fps,
        video_codec: first_vcodec,
        audio_codec: first_acodec,
        audio_channels: first_channels,
        sample_rate: first_sample_rate,
        container,
        bitrate,
        format_name,
        has_video,
        has_audio,
        streams,
        video: video_metadata,
        audio_tracks,
    })
}

fn parse_duration(duration: &Option<String>) -> Option<u64> {
    duration.as_ref().and_then(|d| {
        d.parse::<f64>()
            .ok()
            .map(|seconds| (seconds * 1000.0) as u64)
    })
}

fn parse_fps(r_frame_rate: Option<&str>) -> Option<f32> {
    let rate = r_frame_rate?;
    if rate.contains('/') {
        let parts: Vec<&str> = rate.split('/').collect();
        if parts.len() == 2
            && let (Ok(num), Ok(den)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>())
            && den > 0.0
        {
            return Some(num / den);
        }
    } else if let Ok(val) = rate.parse::<f32>() {
        return Some(val);
    }
    None
}

fn parse_codec_type(codec_type: Option<&str>) -> CodecType {
    match codec_type {
        Some("video") => CodecType::Video,
        Some("audio") => CodecType::Audio,
        Some("subtitle") => CodecType::Subtitle,
        Some("data") => CodecType::Data,
        Some("attachment") => CodecType::Attachment,
        _ => CodecType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffprobe::dto::{FfprobeFormat, FfprobeStream};
    use std::collections::HashMap;

    #[test]
    fn test_parse_ffprobe_output() {
        let mut disposition = HashMap::new();
        disposition.insert("default".to_string(), 1);

        let output = FfprobeOutput {
            streams: vec![
                FfprobeStream {
                    index: 0,
                    codec_type: Some("video".to_string()),
                    codec_name: Some("h264".to_string()),
                    codec_long_name: Some("H.264".to_string()),
                    profile: None,
                    width: Some(1920),
                    height: Some(1080),
                    pix_fmt: Some("yuv420p".to_string()),
                    r_frame_rate: Some("60/1".to_string()),
                    avg_frame_rate: None,
                    sample_rate: None,
                    channels: None,
                    channel_layout: None,
                    bit_rate: None,
                    duration: Some("5.0".to_string()),
                    disposition: HashMap::new(),
                    tags: HashMap::new(),
                },
                FfprobeStream {
                    index: 1,
                    codec_type: Some("audio".to_string()),
                    codec_name: Some("aac".to_string()),
                    codec_long_name: Some("AAC".to_string()),
                    profile: None,
                    width: None,
                    height: None,
                    pix_fmt: None,
                    r_frame_rate: None,
                    avg_frame_rate: None,
                    sample_rate: Some("48000".to_string()),
                    channels: Some(2),
                    channel_layout: Some("stereo".to_string()),
                    bit_rate: None,
                    duration: Some("5.0".to_string()),
                    disposition,
                    tags: HashMap::new(),
                },
            ],
            format: FfprobeFormat {
                format_name: Some("mp4".to_string()),
                format_long_name: Some("MP4 Format".to_string()),
                duration: Some("5.00".to_string()),
                size: Some("12345".to_string()),
                bit_rate: Some("5000000".to_string()),
                tags: HashMap::new(),
            },
        };

        let metadata = parse_ffprobe_output(&output).unwrap();
        assert_eq!(metadata.duration_ms, 5000);
        assert_eq!(metadata.container, Some("mp4".to_string()));
        assert!(metadata.has_video);
        assert!(metadata.has_audio);
        assert_eq!(metadata.video.unwrap().width, 1920);
        assert_eq!(metadata.audio_tracks.len(), 1);
        assert_eq!(metadata.audio_tracks[0].channels, Some(2));
    }
}
