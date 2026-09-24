import { useState } from 'react';
import { editTranslationResult, publishTranslationResult } from '@/entities/translation';
import type { ComparedTranslationSegment, TranslationPublication } from '@/shared/api/contracts/translation';
import { Button } from '@/shared/ui/button';

type EditAttempt = {
  resultId: string;
  lines: string[];
  edited: boolean;
};

type Props = {
  projectId: string;
  translationId: string;
  baseResultId: string;
  segment: ComparedTranslationSegment;
  onPublished: (publication: TranslationPublication) => void;
};

export function TranslationSegmentEditor({
  projectId,
  translationId,
  baseResultId,
  segment,
  onPublished,
}: Props) {
  const [open, setOpen] = useState(false);
  const [lines, setLines] = useState(segment.translatedLines);
  const [attempt, setAttempt] = useState<EditAttempt | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [publication, setPublication] = useState<TranslationPublication | null>(null);

  const save = async () => {
    const current = attempt ?? { resultId: crypto.randomUUID(), lines: [...lines], edited: false };
    setAttempt(current);
    setSaving(true);
    setError(null);
    try {
      if (!current.edited) {
        const savedId = await editTranslationResult({
          projectId,
          translationId,
          baseResultId,
          resultId: current.resultId,
          segmentId: segment.id,
          lines: current.lines,
        });
        if (savedId !== current.resultId) throw new Error('Сохранённая версия не совпадает с запросом');
        setAttempt({ ...current, edited: true });
      }
      const published = await publishTranslationResult(projectId, translationId, current.resultId);
      setPublication(published);
      onPublished(published);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Не удалось сохранить правку');
    } finally {
      setSaving(false);
    }
  };

  if (!open) {
    return <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>Исправить перевод</Button>;
  }

  return (
    <div className="mt-2 space-y-2 border-t border-border pt-2">
      {lines.map((line, index) => (
        <label key={index} className="block text-xs text-muted">
          Строка перевода {index + 1}
          <input
            type="text"
            value={line}
            onChange={(event) => setLines((current) => current.map((value, item) => item === index ? event.target.value : value))}
            disabled={Boolean(attempt) || saving || Boolean(publication)}
            className="mt-1 w-full rounded-sm border border-border bg-canvas px-2 py-1.5 text-xs text-text"
          />
        </label>
      ))}
      {error && <p role="alert" className="text-xs text-danger">{error}</p>}
      {publication && (
        <p role="status" className="text-xs text-muted">
          {publication.state === 'ready'
            ? 'Новая версия готова. Обновите список переводов.'
            : 'Новая версия сохранена. Ожидаем публикации файла; обновите список переводов.'}
        </p>
      )}
      {!publication && (
        <div className="flex gap-2">
          <Button size="sm" onClick={() => void save()} loading={saving} disabled={lines.some((line) => !line.trim())}>
            {attempt ? 'Повторить сохранение' : 'Сохранить новую версию'}
          </Button>
          {!attempt && <Button variant="ghost" size="sm" onClick={() => setOpen(false)}>Отмена</Button>}
        </div>
      )}
    </div>
  );
}
