use crate::media::stream::{AudioTrackMetadata, MediaStream, VideoStreamMetadata};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MediaContainerMetadata {
    pub format_name: Option<String>,
    pub format_long_name: Option<String>,
    pub duration_ms: Option<u64>,
    pub bitrate: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MediaMetadata {
    pub duration_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u8>,
    pub sample_rate: Option<u32>,
    pub container: Option<String>,
    pub bitrate: Option<u64>,
    pub format_name: Option<String>,
    pub has_video: bool,
    pub has_audio: bool,
    pub streams: Vec<MediaStream>,
    pub video: Option<VideoStreamMetadata>,
    pub audio_tracks: Vec<AudioTrackMetadata>,
}
