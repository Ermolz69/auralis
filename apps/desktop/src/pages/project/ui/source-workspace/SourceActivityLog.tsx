import { formatTime, type SourceActivity } from './model';

export function SourceActivityLog({ items }: { items: SourceActivity[] }) {
  return (
    <section aria-labelledby="source-activity-title">
      <h2 id="source-activity-title" className="mb-2 text-xs font-semibold text-muted">
        Лог
      </h2>
      <div className="min-h-[132px] rounded-md border border-border bg-canvas px-3 py-2.5 font-mono text-[11px] leading-5">
        {items.length ? (
          items.map((item, index) => (
            <div key={`${item.timestamp}-${index}`} className="flex min-w-0 gap-3">
              <time dateTime={item.timestamp} className="shrink-0 text-subtle">
                {formatTime(item.timestamp)}
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
    </section>
  );
}
