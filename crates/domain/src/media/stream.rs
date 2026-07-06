#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecType {
    Video,
    Audio,
    Subtitle,
    Data,
    Attachment,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaStream {
    pub index: u32,
    pub codec_type: CodecType,
    pub codec_name: Option<String>,
    pub codec_long_name: Option<String>,
    pub language: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoStreamMetadata {
    pub stream_index: u32,
    pub width: u32,
    pub height: u32,
    pub fps: Option<f32>,
    pub codec: Option<String>,
    pub pixel_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrackMetadata {
    pub stream_index: u32,
    pub codec: Option<String>,
    pub channels: Option<u8>,
    pub channel_layout: Option<String>,
    pub sample_rate: Option<u32>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleStreamMetadata {
    pub stream_index: u32,
    pub codec: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_default: bool,
}
