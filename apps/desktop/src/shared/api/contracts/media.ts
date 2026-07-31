export interface MediaStream {
  index: number;
  codecType: string;
  codecName?: string;
  codecLongName?: string;
  language?: string;
  durationMs?: number;
}

export interface VideoStreamMetadata {
  streamIndex: number;
  width: number;
  height: number;
  fps?: number;
  codec?: string;
  pixelFormat?: string;
}

export interface AudioTrackMetadata {
  streamIndex: number;
  codec?: string;
  channels?: number;
  channelLayout?: string;
  sampleRate?: number;
  language?: string;
  title?: string;
  isDefault: boolean;
}

export interface MediaMetadata {
  durationMs: number;
  width?: number;
  height?: number;
  fps?: number;
  videoCodec?: string;
  audioCodec?: string;
  sampleRate?: number;
  audioChannels?: number;
  container?: string;
  bitrate?: number;
  formatName?: string;
  hasVideo: boolean;
  hasAudio: boolean;
  video?: VideoStreamMetadata;
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
