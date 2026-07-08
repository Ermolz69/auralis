import { formatDuration } from '@/entities/media';
import type { MediaMetadata } from '@/entities/media';

interface MediaSummaryProps {
  metadata: MediaMetadata;
}

export function MediaSummary({ metadata }: MediaSummaryProps) {
  const primaryAudio = metadata.audioTracks?.find((t) => t.isDefault) || metadata.audioTracks?.[0];

  return (
    <div className="flex flex-wrap items-center gap-2 mt-1 text-xs text-muted-foreground">
      <span>{formatDuration(metadata.durationMs)}</span>
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
      {metadata.videoCodec && (
        <>
          <span>•</span>
          <span>{metadata.videoCodec}</span>
        </>
      )}
      {metadata.audioTracks && metadata.audioTracks.length > 0 && (
        <>
          <span>•</span>
          <span>
            {metadata.audioTracks.length} audio track
            {metadata.audioTracks.length !== 1 ? 's' : ''}
          </span>
        </>
      )}
      {primaryAudio && (
        <>
          <span>•</span>
          <span>
            {primaryAudio.codec} / {primaryAudio.channels}ch / {primaryAudio.sampleRate}Hz /{' '}
            {primaryAudio.language || 'unk'}
          </span>
        </>
      )}
    </div>
  );
}
