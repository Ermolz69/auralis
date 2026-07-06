use domain::media::{MediaMetadata, MediaSource};
use serde::Serialize;

#[derive(Serialize)]
pub struct MediaSourceDto {
    pub kind: String,
    pub url_or_path: String,
}

impl From<&MediaSource> for MediaSourceDto {
    fn from(m: &MediaSource) -> Self {
        match m {
            MediaSource::RemoteUrl { url } => Self {
                kind: "RemoteUrl".to_string(),
                url_or_path: url.clone(),
            },
            MediaSource::YoutubeUrl { url } => Self {
                kind: "YoutubeUrl".to_string(),
                url_or_path: url.clone(),
            },
            MediaSource::LocalFile { path } => Self {
                kind: "LocalFile".to_string(),
                url_or_path: path.clone(),
            },
        }
    }
}

use domain::media::stream::{AudioTrackMetadata, MediaStream, VideoStreamMetadata};

#[derive(Serialize)]
pub struct MediaStreamDto {
    pub index: u32,
    pub codec_type: String,
    pub codec_name: Option<String>,
    pub codec_long_name: Option<String>,
    pub language: Option<String>,
    pub duration_ms: Option<u64>,
}

impl From<&MediaStream> for MediaStreamDto {
    fn from(s: &MediaStream) -> Self {
        Self {
            index: s.index,
            codec_type: format!("{:?}", s.codec_type),
            codec_name: s.codec_name.clone(),
            codec_long_name: s.codec_long_name.clone(),
            language: s.language.clone(),
            duration_ms: s.duration_ms,
        }
    }
}

#[derive(Serialize)]
pub struct VideoStreamMetadataDto {
    pub stream_index: u32,
    pub width: u32,
    pub height: u32,
    pub fps: Option<f32>,
    pub codec: Option<String>,
    pub pixel_format: Option<String>,
}

impl From<&VideoStreamMetadata> for VideoStreamMetadataDto {
    fn from(v: &VideoStreamMetadata) -> Self {
        Self {
            stream_index: v.stream_index,
            width: v.width,
            height: v.height,
            fps: v.fps,
            codec: v.codec.clone(),
            pixel_format: v.pixel_format.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct AudioTrackMetadataDto {
    pub stream_index: u32,
    pub codec: Option<String>,
    pub channels: Option<u8>,
    pub channel_layout: Option<String>,
    pub sample_rate: Option<u32>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
}

impl From<&AudioTrackMetadata> for AudioTrackMetadataDto {
    fn from(a: &AudioTrackMetadata) -> Self {
        Self {
            stream_index: a.stream_index,
            codec: a.codec.clone(),
            channels: a.channels,
            channel_layout: a.channel_layout.clone(),
            sample_rate: a.sample_rate,
            language: a.language.clone(),
            title: a.title.clone(),
            is_default: a.is_default,
        }
    }
}

#[derive(Serialize)]
pub struct MediaMetadataDto {
    pub duration_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub audio_channels: Option<u8>,
    pub container: Option<String>,
    pub bitrate: Option<u64>,
    pub format_name: Option<String>,
    pub has_video: bool,
    pub has_audio: bool,
    pub streams: Vec<MediaStreamDto>,
    pub video: Option<VideoStreamMetadataDto>,
    pub audio_tracks: Vec<AudioTrackMetadataDto>,
}

impl From<&MediaMetadata> for MediaMetadataDto {
    fn from(m: &MediaMetadata) -> Self {
        Self {
            duration_ms: m.duration_ms,
            width: m.width,
            height: m.height,
            fps: m.fps,
            video_codec: m.video_codec.clone(),
            audio_codec: m.audio_codec.clone(),
            sample_rate: m.sample_rate,
            audio_channels: m.audio_channels,
            container: m.container.clone(),
            bitrate: m.bitrate,
            format_name: m.format_name.clone(),
            has_video: m.has_video,
            has_audio: m.has_audio,
            streams: m.streams.iter().map(Into::into).collect(),
            video: m.video.as_ref().map(Into::into),
            audio_tracks: m.audio_tracks.iter().map(Into::into).collect(),
        }
    }
}
