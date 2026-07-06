export interface MediaStream {
  index: number;
  codec_type: string;
  codec_name?: string;
  codec_long_name?: string;
  language?: string;
  duration_ms?: number;
}

export interface VideoStreamMetadata {
  stream_index: number;
  width: number;
  height: number;
  fps?: number;
  codec?: string;
  pixel_format?: string;
}

export interface AudioTrackMetadata {
  stream_index: number;
  codec?: string;
  channels?: number;
  channel_layout?: string;
  sample_rate?: number;
  language?: string;
  title?: string;
  is_default: boolean;
}

export interface MediaMetadata {
  duration_ms: number;
  width?: number;
  height?: number;
  fps?: number;
  video_codec?: string;
  audio_codec?: string;
  sample_rate?: number;
  audio_channels?: number;
  container?: string;
  bitrate?: number;
  format_name?: string;
  has_video: boolean;
  has_audio: boolean;
  video?: VideoStreamMetadata;
  audio_tracks: AudioTrackMetadata[];
  streams: MediaStream[];
}

export interface MediaSource {
  kind: string;
  url_or_path: string;
}
