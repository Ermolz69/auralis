import { formatDuration } from '@/entities/media';
import type { MediaMetadata } from '@/entities/media';

interface MediaSummaryProps {
  metadata: MediaMetadata;
}

export function MediaSummary({ metadata }: MediaSummaryProps) {
  const primaryAudio =
    metadata.audioTracks?.find((track) => track.isDefault) ?? metadata.audioTracks?.[0];

  return (
    <div className="mt-1 flex flex-wrap items-center gap-2 font-mono text-[10px] text-muted">
      <span>{formatDuration(metadata.durationMs)}</span>
      {metadata.width && metadata.height && (
        <>
          <span aria-hidden="true">·</span>
          <span>
            {metadata.width}×{metadata.height}
          </span>
        </>
      )}
      <span aria-hidden="true">·</span>
      <span>{metadata.container?.toUpperCase() || 'UNKNOWN'}</span>
      {metadata.videoCodec && (
        <>
          <span aria-hidden="true">·</span>
          <span>{metadata.videoCodec.toUpperCase()}</span>
        </>
      )}
      {metadata.audioTracks.length > 0 && (
        <>
          <span aria-hidden="true">·</span>
          <span>
            {metadata.audioTracks.length} audio track{metadata.audioTracks.length === 1 ? '' : 's'}
          </span>
        </>
      )}
      {primaryAudio && (
        <>
          <span aria-hidden="true">·</span>
          <span>
            {primaryAudio.codec?.toUpperCase() || 'UNKNOWN'} / {primaryAudio.channels ?? '?'}ch /{' '}
            {primaryAudio.sampleRate ?? '?'}Hz / {primaryAudio.language || 'und'}
          </span>
        </>
      )}
    </div>
  );
}
