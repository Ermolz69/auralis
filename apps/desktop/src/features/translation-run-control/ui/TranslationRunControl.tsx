import { useEffect, useRef, useState } from 'react';
import {
  listTranslationHistory,
  pauseTranslationRun,
  publishTranslationResult,
  startTranslationRun,
} from '@/entities/translation';
import type { TranslationLink, TranslationRunSnapshot } from '@/shared/api/contracts/translation';
import { toCommandError } from '@/shared/api/contracts';
import { Button } from '@/shared/ui/button';
import { observeTranslationRuns } from '../model/observeTranslationRuns';

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

type Props = { projectId: string; sourceArtifactId?: string; onReadyResultsChanged?: () => void };

export function TranslationRunControl({
  projectId,
  sourceArtifactId,
  onReadyResultsChanged,
}: Props) {
  const [links, setLinks] = useState<TranslationLink[]>([]);
  const [snapshots, setSnapshots] = useState<Record<string, TranslationRunSnapshot>>({});
  const [busyId, setBusyId] = useState<string | null>(null);
  const [startingId, setStartingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const readyResults = useRef('');

  useEffect(() => {
    let active = true;
    let observation = 0;
    const dispose = observeTranslationRuns(projectId, {
      onSnapshot: (all, statuses) => {
        const current = sourceArtifactId
          ? all.filter((link) => link.sourceArtifactId === sourceArtifactId)
          : all;
        const selectedResults = current
          .filter((link) => link.selectedResultId && link.outputArtifactId)
          .map((link) => `${link.translationId}:${link.selectedResultId}`)
          .sort();
        setLinks(current);
        setSnapshots(statuses);
        const currentObservation = ++observation;
        void Promise.all(
          current.map(async (link) => {
            const page = await listTranslationHistory(projectId, link.translationId, null, 1);
            const latest = page.entries[0]?.resultId;
            return latest ? `${link.translationId}:${latest}` : null;
          }),
        )
          .then((historyHeads) => {
            if (!active || currentObservation !== observation) return;
            const currentReadyResults = [
              ...selectedResults,
              ...historyHeads.filter((head) => head !== null),
            ]
              .sort()
              .join('|');
            if (readyResults.current !== currentReadyResults) onReadyResultsChanged?.();
            readyResults.current = currentReadyResults;
          })
          .catch((reason) => {
            if (active && currentObservation === observation) setError(failureMessage(reason));
          });
      },
      onError: (reason) => setError(failureMessage(reason)),
    });
    return () => {
      active = false;
      dispose();
    };
  }, [projectId, sourceArtifactId, revision, onReadyResultsChanged]);

  const start = async (translationId: string) => {
    if (busyId) return;
    setBusyId(translationId);
    setStartingId(translationId);
    setError(null);
    try {
      const accepted = await startTranslationRun(projectId, translationId);
      setSnapshots((current) => {
        const previous = current[translationId];
        if (!previous || previous.runId !== accepted.runId) return current;
        return {
          ...current,
          [translationId]: { ...previous, activeHostJobId: accepted.jobId },
        };
      });
    } catch (reason) {
      if (toCommandError(reason).code !== 'CANCELLED') setError(failureMessage(reason));
    } finally {
      setRevision((value) => value + 1);
      setStartingId(null);
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
    <section
      className="rounded-md border border-border bg-surface p-4"
      aria-label="Перевод субтитров"
    >
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold text-text">Перевод субтитров</h2>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setError(null);
            setRevision((value) => value + 1);
          }}
        >
          Обновить перевод
        </Button>
      </div>
      {error && (
        <p role="alert" className="mt-2 text-xs text-danger">
          {error}
        </p>
      )}
      <ul className="mt-3 space-y-2 text-xs text-text">
        {links.map((link) => {
          const snapshot = snapshots[link.translationId];
          const preparing =
            startingId === link.translationId ||
            (Boolean(snapshot?.activeHostJobId) &&
              snapshot &&
              ['requested', 'paused', 'failed'].includes(snapshot.phase));
          const canStart =
            snapshot && !preparing && ['requested', 'paused', 'failed'].includes(snapshot.phase);
          const canPause =
            snapshot && (snapshot.phase === 'running' || preparing) && !snapshot.pauseRequested;
          const unpublishedResultId =
            snapshot?.phase === 'validated' && snapshot.resultId !== link.selectedResultId
              ? snapshot.resultId
              : null;
          return (
            <li
              key={link.translationId}
              className="flex flex-wrap items-center justify-between gap-2 rounded-sm border border-border px-2 py-1.5"
            >
              <span className="font-mono">{link.translationId.slice(0, 8)}</span>
              <span role="status" aria-label="Состояние перевода">
                {snapshot
                  ? `${preparing ? 'Подготовка модели' : PHASE_LABELS[snapshot.phase]}: ${snapshot.savedBlocks}/${snapshot.totalBlocks}`
                  : 'Состояние загружается'}
                {snapshot?.pauseRequested ? ' · Запрошена пауза' : ''}
                {link.selectedResultId ? ' · Результат прикреплён' : ''}
              </span>
              {canStart && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => void start(link.translationId)}
                  loading={busyId === link.translationId}
                >
                  {snapshot.phase === 'requested' ? 'Начать перевод' : 'Продолжить перевод'}
                </Button>
              )}
              {canPause && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => void pause(link.translationId)}
                >
                  Пауза
                </Button>
              )}
              {unpublishedResultId && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => void publish(link.translationId, unpublishedResultId)}
                  loading={busyId === link.translationId}
                >
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
