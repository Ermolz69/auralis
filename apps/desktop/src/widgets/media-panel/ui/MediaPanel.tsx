import { useProjectContext } from '@/entities/project';
import { formatDuration } from '@/entities/media';
import { AudioTracksList } from './AudioTracksList';
import { StreamsTable } from './StreamsTable';

export function MediaPanel() {
  const { project } = useProjectContext();
  const metadata = project?.metadata;

  if (!metadata) {
    return (
      <div className="p-4 bg-surface border-l border-muted flex flex-col h-full overflow-y-auto w-80">
        <h3 className="font-semibold mb-4">Media Info</h3>
        <p className="text-sm text-muted-foreground">No metadata available</p>
      </div>
    );
  }

  return (
    <div className="p-4 bg-surface border-l border-muted flex flex-col h-full overflow-y-auto w-80 shrink-0">
      <h3 className="font-semibold mb-4 text-lg">Media</h3>

      <div className="space-y-6">
        <div className="space-y-2">
          <div className="flex justify-between text-sm">
            <span className="text-muted-foreground">Duration:</span>
            <span className="font-medium">{formatDuration(metadata.duration_ms)}</span>
          </div>
          {metadata.width && metadata.height && (
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Resolution:</span>
              <span className="font-medium">
                {metadata.width}×{metadata.height}
              </span>
            </div>
          )}
          {metadata.video_codec && (
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Video:</span>
              <span className="font-medium">
                {metadata.video_codec}
                {metadata.fps ? `, ${metadata.fps.toFixed(2)} fps` : ''}
              </span>
            </div>
          )}
        </div>

        <div>
          <h4 className="font-medium text-sm mb-2 text-foreground/80">Audio tracks:</h4>
          <AudioTracksList tracks={metadata.audio_tracks || []} />
        </div>

        <div>
          <h4 className="font-medium text-sm mb-2 text-foreground/80">Streams:</h4>
          <StreamsTable streams={metadata.streams || []} />
        </div>
      </div>
    </div>
  );
}
