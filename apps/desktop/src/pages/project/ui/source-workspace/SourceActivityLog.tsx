import { formatClockTime } from '@/shared/lib';
import type { SourceActivity } from './model';
import { WorkspaceSection } from '../WorkspaceSection';

export function SourceActivityLog({ items }: { items: SourceActivity[] }) {
  return (
    <WorkspaceSection title="Лог">
      <div className="min-h-[132px] rounded-md border border-border bg-canvas px-3 py-2.5 font-mono text-[11px] leading-5">
        {items.length ? (
          items.map((item, index) => (
            <div key={`${item.timestamp}-${index}`} className="flex min-w-0 gap-3">
              <time dateTime={item.timestamp} className="shrink-0 text-subtle">
                {formatClockTime(item.timestamp)}
              </time>
              <span
                className={`min-w-0 break-words ${item.tone === 'danger' ? 'text-danger' : item.tone === 'success' ? 'text-text' : 'text-muted'}`}
              >
                {item.text}
              </span>
            </div>
          ))
        ) : (
          <p className="text-subtle">Событий пока нет</p>
        )}
      </div>
    </WorkspaceSection>
  );
}
