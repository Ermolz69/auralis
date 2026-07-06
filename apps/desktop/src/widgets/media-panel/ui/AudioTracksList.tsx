import type { AudioTrackMetadata } from '@/entities/media';

interface AudioTracksListProps {
  tracks: AudioTrackMetadata[];
}

export function AudioTracksList({ tracks }: AudioTracksListProps) {
  if (!tracks || tracks.length === 0) {
    return <div className="text-sm text-muted-foreground">No audio tracks found</div>;
  }

  return (
    <div className="flex flex-col gap-1">
      {tracks.map((t, idx) => (
        <div key={t.stream_index} className="text-sm flex items-center gap-2">
          <span className="text-muted-foreground w-6">#{idx}</span>
          <span className="font-medium">{t.codec}</span>
          <span className="text-muted-foreground">
            {t.channels} channels, {t.sample_rate} Hz, {t.language || 'unknown'}
            {t.is_default ? ', default' : ''}
          </span>
        </div>
      ))}
    </div>
  );
}
