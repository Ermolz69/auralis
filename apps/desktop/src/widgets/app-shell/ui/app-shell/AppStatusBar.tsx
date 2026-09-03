import { formatDuration } from '@/entities/media';

type Props = {
  projectTitle: string | null;
  durationMs?: number;
  width?: number;
  height?: number;
  activeJobTitle?: string;
  activeJobPercent?: number;
};

export function AppStatusBar(props: Props) {
  return (
    <footer className="hidden h-7 shrink-0 items-center justify-between border-t border-border/70 bg-canvas font-mono text-[10px] text-subtle lg:flex">
      <div className="flex h-full items-center">
        <span className="flex h-full items-center border-r border-border/70 px-2">
          v0.1.0 · local
        </span>
        <span className="max-w-72 truncate border-r border-border/70 px-3">
          {props.projectTitle ?? 'Auralis Signal'}
        </span>
        {props.projectTitle && props.width && props.height && (
          <span className="border-r border-border/70 px-3">
            {props.width}×{props.height}
          </span>
        )}
        {props.projectTitle && props.durationMs !== undefined && (
          <span className="px-3">{formatDuration(props.durationMs)}</span>
        )}
      </div>
      <div className="flex h-full items-center">
        {props.activeJobTitle && (
          <span className="flex h-full items-center gap-2 border-l border-border/70 px-3 text-primary">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-primary" />
            <span className="max-w-56 truncate">{props.activeJobTitle}</span>
            {Number.isFinite(props.activeJobPercent) && <span>{props.activeJobPercent}%</span>}
          </span>
        )}
        <span className="flex h-full items-center border-l border-border/70 px-3">MP4</span>
        <span className="flex h-full items-center border-l border-border/70 px-3">UTF-8</span>
        <span className="flex h-full items-center border-l border-border/70 px-3">
          Рабочее пространство
        </span>
      </div>
    </footer>
  );
}
