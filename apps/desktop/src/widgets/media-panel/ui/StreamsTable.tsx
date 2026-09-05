import type { MediaStream } from '@/entities/media';
import { Icon, type IconName } from '@/shared/ui/icon';

interface StreamsTableProps {
  streams: MediaStream[];
}

export function StreamsTable({ streams }: StreamsTableProps) {
  if (!streams || streams.length === 0) {
    return <div className="text-sm text-muted p-3 bg-muted/20 rounded-md">No streams found</div>;
  }

  const getStreamIcon = (type: string): IconName => {
    switch (type.toLowerCase()) {
      case 'video':
        return 'Video';
      case 'audio':
        return 'Music';
      case 'subtitle':
        return 'Subtitles';
      default:
        return 'Settings2';
    }
  };

  return (
    <div className="flex flex-col gap-2">
      {streams.map((s) => (
        <div
          key={s.index}
          className="flex flex-col gap-1.5 p-3 bg-surface border border-muted rounded-lg shadow-sm"
        >
          <div className="flex items-center justify-between gap-2">
            <div className="flex min-w-0 items-center gap-2 text-text">
              <div className="p-1 bg-muted/50 rounded text-muted">
                <Icon name={getStreamIcon(s.codecType)} size={14} />
              </div>
              <span className="truncate text-sm font-medium capitalize">{s.codecType} Stream</span>
            </div>
            <span className="text-xs font-medium text-text bg-secondary px-1.5 py-0.5 rounded">
              #{s.index}
            </span>
          </div>
          <div className="flex min-w-0 flex-col ml-8 text-xs text-muted">
            <span className="truncate font-medium text-text">
              {s.codecName?.toUpperCase()} {s.codecLongName ? `(${s.codecLongName})` : ''}
            </span>
            {s.language && <span>Language: {s.language.toUpperCase()}</span>}
          </div>
        </div>
      ))}
    </div>
  );
}
