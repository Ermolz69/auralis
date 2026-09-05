export interface MediaStream {
  index: number;
  codecType: string;
  codecName?: string | null;
  codecLongName?: string | null;
  language?: string | null;
  durationMs?: number | null;
}

export interface VideoStreamMetadata {
  streamIndex: number;
  width: number;
  height: number;
  fps?: number | null;
  codec?: string | null;
  pixelFormat?: string | null;
}

export interface AudioTrackMetadata {
  streamIndex: number;
  codec?: string | null;
  channels?: number | null;
  channelLayout?: string | null;
  sampleRate?: number | null;
  language?: string | null;
  title?: string | null;
  isDefault: boolean;
}

export interface MediaMetadata {
  durationMs: number;
  width?: number | null;
  height?: number | null;
  fps?: number | null;
  videoCodec?: string | null;
  audioCodec?: string | null;
  sampleRate?: number | null;
  audioChannels?: number | null;
  container?: string | null;
  bitrate?: number | null;
  formatName?: string | null;
  hasVideo: boolean;
  hasAudio: boolean;
  video?: VideoStreamMetadata | null;
  audioTracks: AudioTrackMetadata[];
  streams: MediaStream[];
}

export type MediaSourceKind = 'managedLocalFile' | 'youtubeUrl' | 'remoteUrl' | 'externalLocalFile';

export type MediaSource =
  | {
      kind: 'managedLocalFile';
      artifactId: string;
      originalFilename: string;
    }
  | {
      kind: 'youtubeUrl';
      url: string;
    }
  | {
      kind: 'remoteUrl';
      url: string;
    }
  | {
      kind: 'externalLocalFile';
      path: string;
    };
