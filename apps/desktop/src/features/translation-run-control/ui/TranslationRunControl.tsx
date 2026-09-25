import { useEffect, useRef, useState } from 'react';
import {
  getTranslationRunStatus,
  listProjectTranslations,
  pauseTranslationRun,
  publishTranslationResult,
  startTranslationRun,
} from '@/entities/translation';
import type { TranslationLink, TranslationRunSnapshot } from '@/shared/api/contracts/translation';
import { toCommandError } from '@/shared/api/contracts';
import { Button } from '@/shared/ui/button';

const REFRESH_INTERVAL_MS = 3000;
const PHASE_LABELS: Record<TranslationRunSnapshot['phase'], string> = {
  requested: 'Ожидает запуска',
  running: 'Переводится',
  paused: 'На паузе',
  failed: 'Ошибка',
  validated: 'Переведён',
};

function failureMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : toCommandError(reason).message;
}

type Props = { projectId: string; onReadyResultsChanged?: () => void };

export function TranslationRunControl({ projectId, onReadyResultsChanged }: Props) {
  const [links, setLinks] = useState<TranslationLink[]>([]);
  const [snapshots, setSnapshots] = useState<Record<string, TranslationRunSnapshot>>({});
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const readyResults = useRef('');

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      try {
        const current = await listProjectTranslations(projectId);
        const statuses = await Promise.all(current.map(async (link) => {
          if (!link.activeRunId) return null;
          return [link.translationId, await getTranslationRunStatus(projectId, link.translationId)] as const;
        }));
        if (active) {
          const currentReadyResults = current
            .filter((link) => link.selectedResultId && link.outputArtifactId)
            .map((link) => `${link.translationId}:${link.selectedResultId}`)
            .sort()
            .join('|');
          if (readyResults.current !== currentReadyResults) {
            onReadyResultsChanged?.();
          }
          readyResults.current = currentReadyResults;
          setLinks(current);
          setSnapshots(Object.fromEntries(statuses.filter((item) => item !== null)));
        }
      } catch (reason) {
        if (active) setError(failureMessage(reason));
      } finally {
        if (active) timer = setTimeout(() => void poll(), REFRESH_INTERVAL_MS);
      }
    };
    void poll();
    return () => {
      active = false;
      if (timer) clearTimeout(timer);
    };
  }, [projectId, revision, onReadyResultsChanged]);

  const start = async (translationId: string) => {
    if (busyId) return;
    setBusyId(translationId);
    setError(null);
    try {
      await startTranslationRun(projectId, translationId);
      setRevision((value) => value + 1);
    } catch (reason) {
      setError(failureMessage(reason));
    } finally {
      setBusyId(null);
    }
  };

  const pause = async (translationId: string) => {
    setError(null);
    try {
      await pauseTranslationRun(projectId, translationId);
      setRevision((value) => value + 1);
    } catch (reason) {
      setError(failureMessage(reason));
    }
  };

  const publish = async (translationId: string, resultId: string) => {
    if (busyId) return;
    setBusyId(translationId);
    setError(null);
    try {
      await publishTranslationResult(projectId, translationId, resultId);
      setRevision((value) => value + 1);
    } catch (reason) {
      setError(failureMessage(reason));
    } finally {
      setBusyId(null);
    }
  };

  if (links.length === 0 && !error) return null;

  return (
    <section className="rounded-md border border-border bg-surface p-4" aria-label="Перевод субтитров">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-text">Перевод субтитров</h2>
        <Button variant="ghost" size="sm" onClick={() => { setError(null); setRevision((value) => value + 1); }}>Обновить перевод</Button>
      </div>
      {error && <p role="alert" className="mt-2 text-xs text-danger">{error}</p>}
      <ul className="mt-3 space-y-2 text-xs text-text">
        {links.map((link) => {
          const snapshot = snapshots[link.translationId];
          const canStart = snapshot && ['requested', 'paused', 'failed'].includes(snapshot.phase);
          const canPause = snapshot?.phase === 'running' && !snapshot.pauseRequested;
          const unpublishedResultId = snapshot?.phase === 'validated'
            && snapshot.resultId !== link.selectedResultId ? snapshot.resultId : null;
          return (
            <li key={link.translationId} className="flex flex-wrap items-center justify-between gap-2 rounded-sm border border-border px-2 py-1.5">
              <span className="font-mono">{link.translationId.slice(0, 8)}</span>
              <span>
                {snapshot ? `${PHASE_LABELS[snapshot.phase]}: ${snapshot.savedBlocks}/${snapshot.totalBlocks}` : 'Состояние загружается'}
                {snapshot?.pauseRequested ? ' · Запрошена пауза' : ''}
                {link.selectedResultId ? ' · Результат прикреплён' : ''}
              </span>
              {canStart && (
                <Button size="sm" variant="secondary" onClick={() => void start(link.translationId)} loading={busyId === link.translationId}>
                  {snapshot.phase === 'requested' ? 'Начать перевод' : 'Продолжить перевод'}
                </Button>
              )}
              {canPause && <Button size="sm" variant="secondary" onClick={() => void pause(link.translationId)}>Пауза</Button>}
              {unpublishedResultId && (
                <Button size="sm" variant="secondary" onClick={() => void publish(link.translationId, unpublishedResultId)} loading={busyId === link.translationId}>
                  Прикрепить результат
                </Button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
