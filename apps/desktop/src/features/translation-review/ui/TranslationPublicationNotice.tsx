import { useEffect, useState } from 'react';
import { getTranslationPublication } from '@/entities/translation';
import type { TranslationPublicationView } from '@/shared/api/contracts/translationHistoricalEdit';
import { toCommandError } from '@/shared/api/contracts';
import { Button } from '@/shared/ui/button';

const PUBLICATION_POLL_MS = 3000;

export function TranslationPublicationNotice({
  projectId,
  translationId,
  resultId,
  refreshRevision,
}: {
  projectId: string;
  translationId: string;
  resultId: string;
  refreshRevision: number;
}) {
  const [view, setView] = useState<TranslationPublicationView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let current = true;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    setView(null);
    setError(null);
    const refresh = async () => {
      try {
        const found = await getTranslationPublication(projectId, translationId, resultId);
        if (!current) return;
        setView(found);
        setError(null);
        if (found.publication.state === 'pending')
          timeout = setTimeout(() => void refresh(), PUBLICATION_POLL_MS);
      } catch (reason) {
        if (!current) return;
        setError(reason instanceof Error ? reason.message : toCommandError(reason).message);
        timeout = setTimeout(() => void refresh(), PUBLICATION_POLL_MS);
      }
    };
    void refresh();
    return () => {
      current = false;
      clearTimeout(timeout);
    };
  }, [projectId, translationId, resultId, refreshRevision, revision]);
  const observed =
    view?.link.projectId === projectId &&
    view.link.translationId === translationId &&
    view.publication.resultId === resultId
      ? view
      : null;
  const state = observed?.publication.state;
  const attached = state === 'ready' && observed?.link.selectedResultId === resultId;
  return (
    <div className="mt-3 rounded-sm border border-border p-2 text-xs text-muted">
      <p role="status">
        {state === 'ready'
          ? attached
            ? 'Новая версия готова и выбрана для проекта. Требует проверки.'
            : 'Новая версия готова в истории. Текущая версия проекта сохранена.'
          : state === 'failed'
            ? 'Публикация новой версии не завершена. Требуется проверка.'
            : 'Новая версия сохранена. Ожидаем готовности отдельного файла.'}
      </p>
      {error && (
        <p role="alert" className="text-danger">
          {error}
        </p>
      )}
      <Button variant="ghost" size="sm" onClick={() => setRevision((value) => value + 1)}>
        Проверить готовность версии
      </Button>
    </div>
  );
}
