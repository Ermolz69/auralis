import { formatDuration } from '@/entities/media';
import type { MediaMetadata } from '@/entities/media';

interface MediaSummaryProps {
  metadata: MediaMetadata;
}

export function MediaSummary({ metadata }: MediaSummaryProps) {
  const primaryAudio =
    metadata.audio_tracks?.find((t) => t.is_default) || metadata.audio_tracks?.[0];

  return (
    <div className="flex flex-wrap items-center gap-2 mt-1 text-xs text-muted-foreground">
      <span>{formatDuration(metadata.duration_ms)}</span>
      <span>•</span>
      {metadata.width && metadata.height && (
        <>
          <span>
            {metadata.width}×{metadata.height}
          </span>
          <span>•</span>
        </>
      )}
      <span>{metadata.container?.toUpperCase() || 'UNKNOWN'}</span>
      {metadata.video_codec && (
        <>
          <span>•</span>
          <span>{metadata.video_codec}</span>
        </>
      )}
      {metadata.audio_tracks && metadata.audio_tracks.length > 0 && (
        <>
          <span>•</span>
          <span>
            {metadata.audio_tracks.length} audio track
            {metadata.audio_tracks.length !== 1 ? 's' : ''}
          </span>
        </>
      )}
      {primaryAudio && (
        <>
          <span>•</span>
          <span>
            {primaryAudio.codec} / {primaryAudio.channels}ch / {primaryAudio.sample_rate}Hz /{' '}
            {primaryAudio.language || 'unk'}
          </span>
        </>
      )}
    </div>
  );
}
