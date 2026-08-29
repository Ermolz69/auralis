import { useState } from 'react';
import type { useTranscript } from '@/entities/transcript';
import { Button } from '@/shared/ui/button';
import { Icon } from '@/shared/ui/icon';
import { formatRange } from './model';
import { StatusBox } from './StatusBox';

type Props = {
  transcript: ReturnType<typeof useTranscript>['transcript'];
  isLoading: boolean;
  error: string | null;
  onRefresh: () => void;
};

export function TranscriptResult({ transcript, isLoading, error, onRefresh }: Props) {
  const [query, setQuery] = useState('');
  if (isLoading) return <StatusBox text="Проверяем полученные субтитры…" />;
  if (error) return <StatusBox text={error} danger />;
  if (!transcript) return null;

  const normalized = query.trim().toLocaleLowerCase('ru-RU');
  const matches = normalized
    ? transcript.segments.filter((segment) =>
        segment.sourceText.toLocaleLowerCase('ru-RU').includes(normalized),
      )
    : transcript.segments.slice(0, 3);

  return (
    <section
      className="rounded-md border border-success/60 bg-success/10 p-3"
      aria-label="Полученные субтитры"
    >
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-xs font-semibold text-success">
          <Icon name="CircleCheck" size={14} />
          Субтитры получены
        </div>
        <Button variant="ghost" size="sm" onClick={onRefresh}>
          Обновить
        </Button>
      </div>
      <p className="mt-3 font-mono text-[11px] text-subtle">subtitles/source.vtt</p>
      <div className="mt-3 border-t border-success/25 pt-2">
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Поиск по тексту субтитров…"
          aria-label="Поиск по тексту субтитров"
          className="mb-2 h-8 w-full rounded-sm border border-success/25 bg-canvas/40 px-3 text-[11px] text-text outline-none placeholder:text-subtle focus:border-primary focus:ring-2 focus:ring-primary/20"
        />
        <p className="mb-1.5 text-[10px] text-subtle" aria-live="polite" aria-atomic="true">
          {normalized ? `Найдено реплик: ${matches.length}` : 'Предпросмотр (первые 3 реплики)'}
        </p>
        {matches.slice(0, 50).map((segment) => (
          <div key={segment.id} className="flex gap-2 font-mono text-[11px] leading-5 text-muted">
            <span className="text-primary">{segment.index + 1}</span>
            <span className="text-subtle">{formatRange(segment.startMs, segment.endMs)}</span>
            <span className="min-w-0 break-words text-text">{segment.sourceText}</span>
          </div>
        ))}
        {normalized && matches.length === 0 && (
          <p className="py-2 text-[11px] text-subtle">Совпадений в субтитрах нет</p>
        )}
      </div>
    </section>
  );
}
