#![allow(clippy::unwrap_used, clippy::expect_used)]
use domain::media::{MediaMetadata, VideoStreamMetadata};

use super::dto::YtDlpOutput;
use super::error::YtDlpError;

#[allow(clippy::collapsible_if)]
pub fn parse_ytdlp_metadata(json: &str) -> Result<MediaMetadata, YtDlpError> {
    let output: YtDlpOutput = serde_json::from_str(json).map_err(YtDlpError::ParseFailed)?;

    let duration_ms = (output.duration.unwrap_or(0.0) * 1000.0) as u64;

    let mut best_width = output.width;
    let mut best_height = output.height;
    let mut best_fps = output.fps;
    let mut best_tbr = None;

    if best_width.is_none() || best_height.is_none() {
        for format in &output.formats {
            if format.vcodec.as_deref().unwrap_or("none") != "none" {
                if let (Some(w), Some(h)) = (format.width, format.height) {
                    if w * h > best_width.unwrap_or(0) * best_height.unwrap_or(0) {
                        best_width = Some(w);
                        best_height = Some(h);
                        if let Some(f) = format.fps {
                            best_fps = Some(f);
                        }
                    }
                }
            }
            if let Some(t) = format.tbr {
                if t > best_tbr.unwrap_or(0.0) {
                    best_tbr = Some(t);
                }
            }
        }
    }

    let vcodec = output.vcodec.as_deref().unwrap_or("none");
    let acodec = output.acodec.as_deref().unwrap_or("none");

    let has_video_format = output
        .formats
        .iter()
        .any(|f| f.vcodec.as_deref().unwrap_or("none") != "none");
    let has_audio_format = output
        .formats
        .iter()
        .any(|f| f.acodec.as_deref().unwrap_or("none") != "none");

    let has_video = vcodec != "none" || has_video_format;
    let has_audio = acodec != "none" || has_audio_format;

    let video_codec = if vcodec != "none" {
        Some(vcodec.to_string())
    } else {
        None
    };

    let audio_codec = if acodec != "none" {
        Some(acodec.to_string())
    } else {
        None
    };

    let bitrate = best_tbr.map(|t| (t * 1000.0) as u64);

    let video = if has_video && (best_width.is_some() || best_height.is_some()) {
        Some(VideoStreamMetadata {
            stream_index: 0,
            width: best_width.unwrap_or(0),
            height: best_height.unwrap_or(0),
            fps: best_fps,
            codec: video_codec.clone(),
            pixel_format: None,
        })
    } else {
        None
    };

    let container = output.ext.clone();

    Ok(MediaMetadata {
        duration_ms,
        width: best_width,
        height: best_height,
        fps: best_fps,
        video_codec,
        audio_codec,
        audio_channels: None,
        sample_rate: None,
        container,
        bitrate,
        format_name: output.ext,
        has_video,
        has_audio,
        streams: vec![],
        video,
        audio_tracks: vec![],
    })
}

pub fn parse_subtitle_tracks(value: &serde_json::Value) -> Vec<domain::media::SubtitleTrack> {
    let mut tracks = Vec::new();

    let parse_obj = |obj: &serde_json::Value, auto: bool| -> Vec<domain::media::SubtitleTrack> {
        let mut t = Vec::new();
        if let Some(map) = obj.as_object() {
            for (lang, formats) in map {
                if let Some(format_list) = formats.as_array() {
                    for format in format_list {
                        let ext = format
                            .get("ext")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let name = format
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        t.push(domain::media::SubtitleTrack {
                            id: format!("{}-{}", lang, ext),
                            language: lang.clone(),
                            label: name,
                            format: Some(ext.to_string()),
                            is_auto_generated: auto,
                        });
                    }
                }
            }
        }
        t
    };

    if let Some(subs) = value.get("subtitles") {
        tracks.extend(parse_obj(subs, false));
    }
    if let Some(auto) = value.get("automatic_captions") {
        tracks.extend(parse_obj(auto, true));
    }

    tracks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_youtube_metadata() {
        let json = include_str!("../../tests/fixtures/youtube_basic.json");

        let meta = parse_ytdlp_metadata(json).unwrap();
        assert_eq!(meta.duration_ms, 120500);
        assert_eq!(meta.width, Some(1920));
        assert_eq!(meta.height, Some(1080));
        assert_eq!(meta.fps, Some(60.0));
        assert_eq!(meta.video_codec, Some("avc1.640028".to_string()));
        assert_eq!(meta.audio_codec, Some("mp4a.40.2".to_string()));
        assert!(meta.has_video);
        assert!(meta.has_audio);
        assert_eq!(meta.container, Some("mp4".to_string()));
        assert!(meta.video.is_some());
        let v = meta.video.unwrap();
        assert_eq!(v.width, 1920);
        assert_eq!(v.height, 1080);
        assert_eq!(v.fps, Some(60.0));
    }

    #[test]
    fn handles_missing_width_height() {
        let json = include_str!("../../tests/fixtures/youtube_missing_width_height.json");

        let meta = parse_ytdlp_metadata(json).unwrap();
        assert_eq!(meta.width, Some(1280));
        assert_eq!(meta.height, Some(720));
        assert_eq!(meta.fps, Some(30.0));
        assert_eq!(meta.bitrate, Some(1500000));
        assert!(meta.has_video);
    }

    #[test]
    fn handles_none_codecs() {
        let json = include_str!("../../tests/fixtures/youtube_none_codecs.json");

        let meta = parse_ytdlp_metadata(json).unwrap();
        assert_eq!(meta.video_codec, None);
        assert_eq!(meta.audio_codec, None);
        assert!(!meta.has_video);
        assert!(!meta.has_audio);
        assert!(meta.video.is_none());
    }

    #[test]
    fn chooses_format_fallback_resolution() {
        let json = include_str!("../../tests/fixtures/youtube_fallback_resolution.json");

        let meta = parse_ytdlp_metadata(json).unwrap();
        assert_eq!(meta.width, Some(1920));
        assert_eq!(meta.height, Some(1080));
    }

    #[test]
    fn duration_seconds_to_ms() {
        let json = include_str!("../../tests/fixtures/youtube_duration.json");
        let meta = parse_ytdlp_metadata(json).unwrap();
        assert_eq!(meta.duration_ms, 1234);
    }
}
