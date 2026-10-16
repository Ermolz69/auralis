import { useEffect, useState } from 'react';
import {
  getTranslationPublication,
  subscribeTranslationPublicationChanges,
} from '@/entities/translation';
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
    let reading = false;
    let pending = false;
    let unsubscribe: (() => void) | undefined;
    const subscriptions = new AbortController();
    setView(null);
    setError(null);

    const scheduleRecovery = () => {
      clearTimeout(timeout);
      if (current) timeout = setTimeout(refresh, PUBLICATION_POLL_MS);
    };

    const read = async () => {
      try {
        const found = await getTranslationPublication(projectId, translationId, resultId);
        if (current && !pending) {
          setView(found);
          setError(null);
          if (found.publication.state === 'pending') scheduleRecovery();
        }
      } catch (reason) {
        if (current && !pending) {
          setView(null);
          setError(reason instanceof Error ? reason.message : toCommandError(reason).message);
          scheduleRecovery();
        }
      } finally {
        reading = false;
        if (current && pending) {
          pending = false;
          refresh();
        }
      }
    };

    function refresh() {
      if (!current) return;
      clearTimeout(timeout);
      if (reading) {
        pending = true;
        return;
      }
      reading = true;
      void read();
    }

    refresh();
    void subscribeTranslationPublicationChanges(projectId, refresh, subscriptions.signal).then(
      (dispose) => {
        if (!current) dispose();
        else {
          unsubscribe = dispose;
          refresh();
        }
      },
      () => {
        if (current) scheduleRecovery();
      },
    );
    return () => {
      current = false;
      subscriptions.abort();
      clearTimeout(timeout);
      unsubscribe?.();
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
