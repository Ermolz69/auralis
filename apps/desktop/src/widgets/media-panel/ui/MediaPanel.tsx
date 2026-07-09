import { useProjectContext } from '@/entities/project';
import { formatDuration } from '@/entities/media';
import { AudioTracksList } from './AudioTracksList';
import { StreamsTable } from './StreamsTable';
import { AlertCircle, FileVideo, Film, Info } from 'lucide-react';
import { Badge } from '@/shared/ui/badge';

export function MediaPanel() {
  const { project } = useProjectContext();
  const metadata = project?.metadata;
  const source = project?.source;

  if (!metadata) {
    return (
      <div className="p-4 bg-surface border-r border-muted flex flex-col h-full overflow-y-auto w-80 shrink-0">
        <h3 className="font-semibold mb-4 text-text flex items-center gap-2">
          <FileVideo className="w-5 h-5 text-muted" />
          Media Info
        </h3>
        <div className="flex flex-col items-center justify-center h-40 text-center space-y-2 text-muted bg-muted/10 rounded-xl border border-dashed border-muted">
          <Info className="w-6 h-6 opacity-50" />
          <p className="text-sm">No metadata available</p>
        </div>
      </div>
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
    <div className="p-4 bg-surface border-r border-muted flex flex-col h-full overflow-y-auto w-80 shrink-0 custom-scrollbar">
      <h3 className="font-semibold mb-4 text-lg text-text flex items-center gap-2">
        <Film className="w-5 h-5 text-primary" />
        Media Engine
      </h3>

      <div className="space-y-6">
        {/* Warnings */}
        {warnings.length > 0 && (
          <div className="flex flex-col gap-2">
            {warnings.map((warn, i) => (
              <div
                key={i}
                className="flex items-start gap-2 text-warning text-xs p-3 bg-warning/10 border border-warning/30 rounded-lg shadow-sm"
              >
                <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
                <span className="leading-snug">{warn}</span>
              </div>
            ))}
          </div>
        )}

        {/* Basic Properties */}
        <div className="space-y-3">
          <h4 className="font-medium text-sm text-text/80 flex items-center gap-1.5">
            <Info className="w-4 h-4 text-muted" />
            Properties
          </h4>
          <div className="space-y-2 bg-muted/10 p-3 rounded-lg border border-muted/50">
            <div className="flex justify-between items-center text-sm">
              <span className="text-muted">Source</span>
              <span
                className="font-medium text-text truncate max-w-[120px]"
                title={source?.urlOrPath}
              >
                {source?.urlOrPath?.split(/[/\\]/).pop() || 'Unknown'}
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
        <div className="space-y-3">
          <h4 className="font-medium text-sm text-text/80">Raw Streams</h4>
          <StreamsTable streams={metadata.streams || []} />
        </div>
      </div>
    </div>
  );
}
