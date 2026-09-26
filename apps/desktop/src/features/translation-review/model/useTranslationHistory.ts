import { useEffect, useRef, useState } from 'react';
import { listTranslationHistory, selectTranslationResult } from '@/entities/translation';
import type { TranslationHistoryEntry, TranslationLink } from '@/shared/api/contracts/translation';
import { toCommandError } from '@/shared/api/contracts';

const HISTORY_PAGE_SIZE = 25;

export function useTranslationHistory(
  projectId: string | null,
  link: TranslationLink | null,
  refreshRevision: number,
  onSelected: () => void,
) {
  const [entries, setEntries] = useState<TranslationHistoryEntry[]>([]);
  const [observed, setObserved] = useState<TranslationLink | null>(null);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [selecting, setSelecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const translationId = link?.translationId ?? null;
  const linkRevision = link?.revision ?? null;

  useEffect(() => {
    const current = ++generation.current;
    setEntries([]);
    setObserved(null);
    setNextCursor(null);
    setSelecting(false);
    setError(null);
    if (!projectId || !translationId) {
      setLoading(false);
      return;
    }
    setLoading(true);
    void listTranslationHistory(projectId, translationId, null, HISTORY_PAGE_SIZE)
      .then((page) => {
        if (generation.current !== current) return;
        setEntries(page.entries);
        setObserved(page.link);
        setNextCursor(page.nextCursor);
      })
      .catch((reason: unknown) => {
        if (generation.current === current) setError(toCommandError(reason).message);
      })
      .finally(() => {
        if (generation.current === current) setLoading(false);
      });
    return () => {
      generation.current = current + 1;
    };
  }, [projectId, translationId, linkRevision, refreshRevision]);

  const more = async () => {
    if (!projectId || !translationId || !nextCursor || loading || selecting) return;
    const current = generation.current;
    setLoading(true);
    setError(null);
    try {
      const page = await listTranslationHistory(
        projectId,
        translationId,
        nextCursor,
        HISTORY_PAGE_SIZE,
      );
      if (generation.current !== current) return;
      setEntries((previous) => [
        ...new Map([...previous, ...page.entries].map((entry) => [entry.resultId, entry])).values(),
      ]);
      setObserved(page.link);
      setNextCursor(page.nextCursor);
    } catch (reason) {
      if (generation.current === current) setError(toCommandError(reason).message);
    } finally {
      if (generation.current === current) setLoading(false);
    }
  };

  const attach = async (resultId: string) => {
    if (!projectId || !translationId || !observed || selecting || loading) return;
    const current = generation.current;
    setSelecting(true);
    setError(null);
    try {
      const selected = await selectTranslationResult(
        projectId,
        translationId,
        resultId,
        observed.revision,
      );
      if (generation.current !== current) return;
      setObserved(selected);
      onSelected();
    } catch (reason) {
      if (generation.current === current) setError(toCommandError(reason).message);
    } finally {
      if (generation.current === current) setSelecting(false);
    }
  };
  return { entries, observed, nextCursor, loading, selecting, error, more, attach };
}
