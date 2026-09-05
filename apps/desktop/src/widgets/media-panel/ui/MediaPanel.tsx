import { useProjectContext } from '@/entities/project';
import { formatDuration, formatSourceLabel } from '@/entities/media';
import { AudioTracksList } from './AudioTracksList';
import { StreamsTable } from './StreamsTable';
import { Badge } from '@/shared/ui/badge';
import { Icon } from '@/shared/ui/icon';
import { Notice } from '@/shared/ui/notice';
import { StateView } from '@/shared/ui/state-view';

type MediaPanelProps = {
  className?: string;
};

export function MediaPanel({ className = '' }: MediaPanelProps) {
  const { project } = useProjectContext();
  const metadata = project?.metadata;
  const source = project?.source;
  const sourceLabel = formatSourceLabel(source ?? null);
  const baseClass = `flex h-full min-w-0 flex-col overflow-y-auto bg-surface p-4 ${className}`;

  if (!metadata) {
    return (
      <aside className={baseClass} aria-label="Media details">
        <h3 className="mb-4 flex items-center gap-2 text-sm font-semibold text-text">
          <Icon name="FileVideo" size="md" color="muted" />
          Media Info
        </h3>
        <StateView
          icon="Info"
          title="No metadata available"
          description={
            <span className="block max-w-full truncate px-3" aria-label={`Source: ${sourceLabel}`}>
              {sourceLabel}
            </span>
          }
          density="compact"
          className="h-40 rounded-xl border border-dashed border-muted bg-muted/10"
        />
      </aside>
    );
  }

  const warnings: string[] = [];
  if (!metadata.hasVideo) {
    warnings.push('No video stream detected. Audio-only mode.');
  }
  if (!metadata.hasAudio || metadata.audioTracks.length === 0) {
    warnings.push('No audio tracks detected. Dubbing requires audio.');
  }

  return (
    <aside className={baseClass} aria-label="Media details">
      <h3 className="mb-4 flex items-center gap-2 text-sm font-semibold text-text">
        <Icon name="Film" size="md" color="primary" />
        Media Engine
      </h3>

      <div className="space-y-6">
        {/* Warnings */}
        {warnings.length > 0 && (
          <div className="flex flex-col gap-2">
            {warnings.map((warn, i) => (
              <Notice key={i} icon="CircleAlert" tone="warning" role="alert">
                {warn}
              </Notice>
            ))}
          </div>
        )}

        {/* Basic Properties */}
        <div className="space-y-3">
          <h4 className="font-medium text-sm text-text/80 flex items-center gap-1.5">
            <Icon name="Info" size="sm" color="muted" />
            Properties
          </h4>
          <div className="space-y-2 rounded-md border border-border bg-surface-raised p-3">
            <div className="flex justify-between items-center text-sm">
              <span className="text-muted">Source</span>
              <span
                className="font-medium text-text truncate max-w-[120px]"
                aria-label={`Source: ${sourceLabel}`}
              >
                {sourceLabel}
              </span>
            </div>
            <div className="flex justify-between items-center text-sm">
              <span className="text-muted">Duration</span>
              <span className="font-medium text-text">{formatDuration(metadata.durationMs)}</span>
            </div>
            <div className="flex justify-between items-center text-sm">
              <span className="text-muted">Container</span>
              <Badge variant="muted" size="sm">
                {metadata.container?.toUpperCase() || 'UNK'}
              </Badge>
            </div>
            {metadata.width && metadata.height && (
              <div className="flex justify-between items-center text-sm">
                <span className="text-muted">Resolution</span>
                <span className="font-medium text-text">
                  {metadata.width}×{metadata.height}
                </span>
              </div>
            )}
            {metadata.videoCodec && (
              <div className="flex justify-between items-center text-sm">
                <span className="text-muted">Video</span>
                <div className="flex flex-col items-end">
                  <span className="font-medium text-text">{metadata.videoCodec.toUpperCase()}</span>
                  {metadata.fps && (
                    <span className="text-[10px] text-muted">{metadata.fps.toFixed(2)} fps</span>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Audio Tracks */}
        <div className="space-y-3">
          <h4 className="font-medium text-sm text-text/80">Audio Tracks</h4>
          <AudioTracksList tracks={metadata.audioTracks || []} />
        </div>

        {/* Streams Table */}
        <details className="group space-y-3 rounded-md border border-border bg-surface-raised p-3">
          <summary className="motion-control cursor-pointer rounded-sm text-sm font-medium text-text/80 hover:text-text focus:outline-none focus:ring-2 focus:ring-focus focus:ring-offset-2 focus:ring-offset-bg">
            More details
          </summary>
          <div className="mt-3 max-w-full animate-content-in overflow-x-auto">
            <h4 className="sr-only">Raw Streams</h4>
            <StreamsTable streams={metadata.streams || []} />
          </div>
        </details>
      </div>
    </aside>
  );
}
