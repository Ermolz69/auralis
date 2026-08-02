import type { ProjectStatus } from '@/shared/api/contracts/project';
import type { MediaSource } from './types';

export type ProjectStatusTone = 'success' | 'danger' | 'primary' | 'warning' | 'muted';

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
  if (source.kind === 'managedLocalFile') return getSafeFilename(source.originalFilename);
  if (source.kind === 'externalLocalFile') return getSafeFilename(source.path);

  const host = getUrlHost(source.url);
  if (source.kind === 'youtubeUrl') return host ? `YouTube source (${host})` : 'YouTube source';
  return host ? `Remote source (${host})` : 'Remote source';
};

export const formatProjectTitle = (title: string, source: MediaSource | null): string => {
  const trimmedTitle = title.trim();
  if (!trimmedTitle) return 'Untitled Project';
  if (trimmedTitle && !isRawUrl(trimmedTitle) && !looksLikePath(trimmedTitle)) return trimmedTitle;
  if (source?.kind === 'youtubeUrl') return 'YouTube project';
  if (source?.kind === 'remoteUrl') return 'Remote source project';
  if (source?.kind === 'managedLocalFile' || source?.kind === 'externalLocalFile') {
    return formatSourceLabel(source);
  }
  return 'Untitled Project';
};

export const formatProjectStatus = (status: ProjectStatus): string => {
  switch (status) {
    case 'draft':
      return 'Draft';
    case 'source_imported':
      return 'Source imported';
    case 'ready_for_processing':
      return 'Ready for processing';
    case 'processing':
      return 'Processing';
    case 'completed':
      return 'Completed';
    case 'failed':
      return 'Needs attention';
    case 'cancelled':
      return 'Cancelled';
  }
};

export const getProjectStatusTone = (status: ProjectStatus): ProjectStatusTone => {
  switch (status) {
    case 'completed':
      return 'success';
    case 'failed':
      return 'danger';
    case 'processing':
      return 'primary';
    case 'cancelled':
      return 'warning';
    case 'draft':
    case 'source_imported':
    case 'ready_for_processing':
      return 'muted';
  }
};

export const supportsSubtitleImport = (source: MediaSource | null): boolean =>
  source?.kind === 'youtubeUrl' || source?.kind === 'remoteUrl';

const isRawUrl = (value: string): boolean => /^https?:\/\//i.test(value.trim());

const looksLikePath = (value: string): boolean =>
  /^[a-z]:[\\/]/i.test(value.trim()) || value.includes('\\') || value.startsWith('/');

const getSafeFilename = (value: string): string => {
  const filename = value.trim().split(/[/\\]/).filter(Boolean).pop();
  return filename || 'Local file';
};

const getUrlHost = (value: string): string | null => {
  try {
    return new URL(value).host.replace(/^www\./, '');
  } catch {
    return null;
  }
};
