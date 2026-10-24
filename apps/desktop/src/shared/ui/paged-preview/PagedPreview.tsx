import { useState, type ReactNode } from 'react';

type Props<T> = {
  label: string;
  items: readonly T[];
  renderItem: (item: T, index: number) => ReactNode;
};
const sizes = [25, 50, 100] as const;
export function PagedPreview<T>({ label, items, renderItem }: Props<T>) {
  const [page, setPage] = useState(0);
  const [size, setSize] = useState<number>(50);
  const pages = Math.max(1, Math.ceil(items.length / size));
  const current = Math.min(page, pages - 1);
  const start = current * size;
  const end = Math.min(start + size, items.length);
  const button = 'rounded border border-border px-2 py-1 disabled:opacity-40';
  return (
    <section aria-label={label} className="space-y-2">
      <nav aria-label={`Страницы: ${label}`} className="flex flex-wrap items-center gap-2">
        <span aria-live="polite">
          {label} {items.length ? start + 1 : 0}–{end} из {items.length}
        </span>
        <label>
          {label} на странице
          <select
            className="ml-1 rounded border border-border bg-surface p-1 text-text"
            value={size}
            onChange={(event) => {
              const next = Number(event.target.value);
              if (!sizes.some((allowed) => allowed === next)) return;
              setSize(next);
              setPage(0);
            }}
          >
            {sizes.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className={button}
          aria-label={`Предыдущая страница: ${label}`}
          disabled={current === 0}
          onClick={() => setPage(current - 1)}
        >
          Назад
        </button>
        <span>
          Страница {current + 1} из {pages}
        </span>
        <button
          type="button"
          className={button}
          aria-label={`Следующая страница: ${label}`}
          disabled={current + 1 === pages}
          onClick={() => setPage(current + 1)}
        >
          Далее
        </button>
      </nav>
      {items.slice(start, end).map((item, index) => renderItem(item, start + index))}
    </section>
  );
}
