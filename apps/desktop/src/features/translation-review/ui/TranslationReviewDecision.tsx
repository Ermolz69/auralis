import { useEffect, useState } from 'react';
import { getTranslationReview, recordTranslationReview } from '@/entities/translation';
import type {
  TranslationReviewDecision as Decision,
  TranslationReviewView,
} from '@/shared/api/contracts/translation';
import { Button } from '@/shared/ui/button';

export function TranslationReviewDecision({
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
  const [view, setView] = useState<TranslationReviewView | null>(null);
  const [reviewerLabel, setReviewerLabel] = useState('');
  const [note, setNote] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    setView(null);
    setError(null);
    void getTranslationReview(projectId, translationId, resultId)
      .then((found) => {
        if (current) setView(found);
      })
      .catch((reason: unknown) => {
        if (current)
          setError(reason instanceof Error ? reason.message : 'Не удалось загрузить решение');
      });
    return () => {
      current = false;
    };
  }, [projectId, translationId, resultId, refreshRevision]);

  async function decide(decision: Decision) {
    if (!view || saving) return;
    setSaving(true);
    setError(null);
    try {
      const updated = await recordTranslationReview(
        projectId,
        translationId,
        resultId,
        view,
        decision,
        reviewerLabel.trim(),
        note.trim(),
      );
      setView(updated);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Не удалось сохранить решение');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="mt-3 rounded-sm border border-border p-3 text-xs">
      <h3 className="font-medium text-text">Решение о проверке перевода</h3>
      {error && (
        <p role="alert" className="mt-2 text-danger">
          {error}
        </p>
      )}
      {!view && !error && (
        <p role="status" className="mt-2 text-muted">
          Проверяем результат…
        </p>
      )}
      {view && (
        <>
          <p className="mt-2 text-muted">
            {view.latest?.decision === 'approved'
              ? `Одобрено: ${view.latest.reviewerLabel}`
              : view.latest?.decision === 'withdrawn'
                ? `Одобрение отозвано: ${view.latest.reviewerLabel}`
                : 'Решение ещё не принято'}
            {!view.selected && ' · эта версия не выбрана для проекта'}
          </p>
          <p className="mt-1 break-all font-mono text-subtle">SHA-256: {view.resultSha256}</p>
          <p className="mt-1 text-muted">
            Это решение относится только к данной версии. Следующая правка требует отдельного
            одобрения.
          </p>
          <label className="mt-3 block text-muted" htmlFor="translation-reviewer-label">
            Проверяющий
          </label>
          <input
            id="translation-reviewer-label"
            className="mt-1 w-full rounded-sm border border-border bg-canvas px-2 py-1.5 text-text"
            value={reviewerLabel}
            maxLength={120}
            onChange={(event) => setReviewerLabel(event.target.value)}
          />
          <label className="mt-2 block text-muted" htmlFor="translation-review-note">
            Основание решения
          </label>
          <textarea
            id="translation-review-note"
            className="mt-1 w-full rounded-sm border border-border bg-canvas px-2 py-1.5 text-text"
            value={note}
            maxLength={2000}
            onChange={(event) => setNote(event.target.value)}
          />
          <div className="mt-2 flex gap-2">
            <Button
              size="sm"
              onClick={() => void decide('approved')}
              disabled={!reviewerLabel.trim() || !note.trim() || saving}
              loading={saving}
            >
              Одобрить для речи
            </Button>
            {view.latest?.decision === 'approved' && (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => void decide('withdrawn')}
                disabled={!reviewerLabel.trim() || !note.trim() || saving}
              >
                Отозвать одобрение
              </Button>
            )}
          </div>
        </>
      )}
    </div>
  );
}
