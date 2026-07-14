use domain::media::{MediaMetadata, MediaSource};
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MediaSourceDto {
    ManagedLocalFile {
        #[serde(rename = "artifactId")]
        artifact_id: String,
        #[serde(rename = "originalFilename")]
        original_filename: String,
    },
    ExternalLocalFile {
        path: String,
    },
    YoutubeUrl {
        url: String,
    },
    RemoteUrl {
        url: String,
    },
}

impl From<&MediaSource> for MediaSourceDto {
    fn from(m: &MediaSource) -> Self {
        match m {
            MediaSource::ManagedLocalFile {
                artifact_id,
                original_filename,
            } => Self::ManagedLocalFile {
                artifact_id: artifact_id.to_string(),
                original_filename: original_filename.clone(),
            },
            MediaSource::ExternalLocalFile { path } => {
                Self::ExternalLocalFile { path: path.clone() }
            }
            MediaSource::YoutubeUrl { url } => Self::YoutubeUrl { url: url.clone() },
            MediaSource::RemoteUrl { url } => Self::RemoteUrl { url: url.clone() },
        }
    }
}

use domain::media::stream::{AudioTrackMetadata, MediaStream, VideoStreamMetadata};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use domain::media::{ArtifactId, MediaSource};

    #[test]
    fn test_media_source_dto_serialization() {
        let artifact_id = ArtifactId::new();
        let source = MediaSource::ManagedLocalFile {
            artifact_id: artifact_id.clone(),
            original_filename: "test.mp4".to_string(),
        };
        let dto: MediaSourceDto = (&source).into();
        let json = serde_json::to_string(&dto).unwrap();
        assert_eq!(
            json,
            format!(
                r#"{{"kind":"managedLocalFile","artifactId":"{}","originalFilename":"test.mp4"}}"#,
                artifact_id.0
            )
        );

        let source = MediaSource::ExternalLocalFile {
            path: "/a/b.mp4".to_string(),
        };
        let dto: MediaSourceDto = (&source).into();
        let json = serde_json::to_string(&dto).unwrap();
        assert_eq!(json, r#"{"kind":"externalLocalFile","path":"/a/b.mp4"}"#);

        let source = MediaSource::YoutubeUrl {
            url: "https://youtu.be/xxx".to_string(),
        };
        let dto: MediaSourceDto = (&source).into();
        let json = serde_json::to_string(&dto).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"youtubeUrl","url":"https://youtu.be/xxx"}"#
        );

        let source = MediaSource::RemoteUrl {
            url: "https://example.com/video.mp4".to_string(),
        };
        let dto: MediaSourceDto = (&source).into();
        let json = serde_json::to_string(&dto).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"remoteUrl","url":"https://example.com/video.mp4"}"#
        );
    }
}
