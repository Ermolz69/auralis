import type { MediaStream } from '@/entities/media';
import { Video, Music, Settings2, Subtitles } from 'lucide-react';

interface StreamsTableProps {
  streams: MediaStream[];
}

export function StreamsTable({ streams }: StreamsTableProps) {
  if (!streams || streams.length === 0) {
    return (
      <div className="text-sm text-muted-foreground p-3 bg-muted/20 rounded-md">
        No streams found
      </div>
    );
  }

  const getStreamIcon = (type: string) => {
    switch (type.toLowerCase()) {
      case 'video':
        return <Video className="w-3.5 h-3.5" />;
      case 'audio':
        return <Music className="w-3.5 h-3.5" />;
      case 'subtitle':
        return <Subtitles className="w-3.5 h-3.5" />;
      default:
        return <Settings2 className="w-3.5 h-3.5" />;
    }
  };

  return (
    <div className="flex flex-col gap-2">
      {streams.map((s) => (
        <div
          key={s.index}
          className="flex flex-col gap-1.5 p-3 bg-surface border border-muted rounded-lg shadow-sm"
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-text">
              <div className="p-1 bg-muted/50 rounded text-muted-foreground">
                {getStreamIcon(s.codecType)}
              </div>
              <span className="text-sm font-medium capitalize">{s.codecType} Stream</span>
            </div>
            <span className="text-xs font-medium text-muted-foreground bg-muted/30 px-1.5 py-0.5 rounded">
              #{s.index}
            </span>
          </div>
          <div className="flex flex-col ml-8 text-xs text-muted-foreground">
            <span className="font-medium text-text">
              {s.codecName?.toUpperCase()} {s.codecLongName ? `(${s.codecLongName})` : ''}
            </span>
            {s.language && <span>Language: {s.language.toUpperCase()}</span>}
          </div>
        </div>
      ))}
    </div>
  );
}
