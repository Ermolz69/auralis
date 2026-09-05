import type { AudioTrackMetadata } from '@/entities/media';
import { Badge } from '@/shared/ui/badge';
import { Icon } from '@/shared/ui/icon';

interface AudioTracksListProps {
  tracks: AudioTrackMetadata[];
}

export function AudioTracksList({ tracks }: AudioTracksListProps) {
  if (!tracks || tracks.length === 0) {
    return (
      <div className="text-sm text-muted p-3 bg-muted/20 rounded-md">No audio tracks found</div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {tracks.map((t, idx) => (
        <div
          key={t.streamIndex}
          className="flex flex-col gap-1 p-3 bg-surface border border-muted rounded-lg shadow-sm"
        >
          <div className="flex items-center justify-between gap-2">
            <div className="flex min-w-0 items-center gap-2">
              <Icon name="Headphones" size={14} color="muted" />
              <span className="truncate text-sm font-semibold text-text">
                Track #{idx} {t.title ? `(${t.title})` : ''}
              </span>
            </div>
            {t.isDefault && (
              <Badge variant="primary" size="sm">
                Default
              </Badge>
            )}
          </div>
          <div className="flex items-center gap-1.5 text-xs text-muted mt-1 flex-wrap">
            <Badge variant="muted" size="sm">
              {t.codec?.toUpperCase() || 'UNKNOWN'}
            </Badge>
            <span>•</span>
            <span className="flex items-center gap-1">
              <Icon name="Volume2" size={12} /> {t.channels} ch
            </span>
            <span>•</span>
            <span>{t.sampleRate} Hz</span>
            <span>•</span>
            <span className="uppercase">{t.language || 'UND'}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
