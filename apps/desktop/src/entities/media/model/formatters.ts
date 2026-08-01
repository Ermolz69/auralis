import type { MediaSource } from './types';

export const formatDuration = (ms: number): string => {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
};

export const formatCodec = (codec?: string): string => {
  return codec ? codec.toUpperCase() : 'Unknown';
};

export const formatSourceLabel = (source: MediaSource | null): string => {
  if (!source) return 'No media source attached';
  if (source.kind === 'managedLocalFile') return source.originalFilename;
  if (source.kind === 'externalLocalFile') return source.path.split(/[/\\]/).pop() || 'Local file';

  const host = getUrlHost(source.url);
  if (source.kind === 'youtubeUrl') return host ? `YouTube source (${host})` : 'YouTube source';
  return host ? `Remote source (${host})` : 'Remote source';
};

export const formatProjectTitle = (title: string, source: MediaSource | null): string => {
  if (!isRawUrl(title)) return title || 'Untitled Project';
  if (source?.kind === 'youtubeUrl') return 'YouTube project';
  if (source?.kind === 'remoteUrl') return 'Remote source project';
  return 'Untitled Project';
};

export const supportsSubtitleImport = (source: MediaSource | null): boolean =>
  source?.kind === 'youtubeUrl' || source?.kind === 'remoteUrl';

const isRawUrl = (value: string): boolean => /^https?:\/\//i.test(value.trim());

const getUrlHost = (value: string): string | null => {
  try {
    return new URL(value).host.replace(/^www\./, '');
  } catch {
    return null;
  }
};
