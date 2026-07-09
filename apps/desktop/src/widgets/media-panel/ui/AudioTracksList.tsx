import type { AudioTrackMetadata } from '@/entities/media';
import { Badge } from '@/shared/ui/badge';
import { Headphones, Volume2 } from 'lucide-react';

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
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Headphones className="w-3.5 h-3.5 text-muted" />
              <span className="text-sm font-semibold text-text">
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
              <Volume2 className="w-3 h-3" /> {t.channels} ch
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
