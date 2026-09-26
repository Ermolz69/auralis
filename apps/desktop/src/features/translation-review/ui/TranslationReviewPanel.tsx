import { Button } from '@/shared/ui/button';
import { formatSubtitleRange } from '@/shared/lib/formatSubtitleRange';
import { useTranslationReview, COMPARISON_PAGE_SIZE } from '../model/useTranslationReview';
import { TranslationSegmentEditor } from './TranslationSegmentEditor';
import { useTranslationHistory } from '../model/useTranslationHistory';
import { TranslationHistorySelector } from './TranslationHistorySelector';
import { useState } from 'react';
import type { TranslationPublication } from '@/shared/api/contracts/translation';
import { TranslationPublicationNotice } from './TranslationPublicationNotice';

export function TranslationReviewPanel({
  projectId,
  refreshRevision = 0,
}: {
  projectId: string | null;
  refreshRevision?: number;
}) {
  const review = useTranslationReview(projectId, refreshRevision);
  const [publication, setPublication] = useState<{
    projectId: string;
    record: TranslationPublication;
  } | null>(null);
  const history = useTranslationHistory(
    projectId,
    review.selected,
    review.historyRevision,
    review.refresh,
  );
  const page = review.page?.resultId === review.previewResultId ? review.page : null;
  const readyLinks = review.links.filter((link) => link.selectedResultId);
  const canEdit = Boolean(page && review.selected);

  return (
    <section
      className="rounded-md border border-border bg-surface p-4"
      aria-label="Переводы проекта"
    >
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-text">Переводы проекта</h2>
        <Button variant="ghost" size="sm" onClick={review.refresh} loading={review.loadingLinks}>
          Обновить переводы
        </Button>
      </div>
      {review.error && (
        <p role="alert" className="mt-3 text-xs text-danger">
          {review.error}
        </p>
      )}
      {!review.loadingLinks && review.links.length === 0 && (
        <p className="mt-3 text-xs text-muted">Пока нет переводов для этого проекта.</p>
      )}
      {publication &&
        projectId === publication.projectId &&
        review.selected?.translationId === publication.record.translationId && (
          <TranslationPublicationNotice
            key={`${projectId}:${publication.record.resultId}`}
            projectId={projectId}
            translationId={publication.record.translationId}
            resultId={publication.record.resultId}
            refreshRevision={review.historyRevision}
          />
        )}
      {review.links.length > 0 && (
        <p className="mt-3 text-xs text-muted">
          {readyLinks.length} готово · {review.links.length - readyLinks.length} в работе
        </p>
      )}
      {readyLinks.length > 0 && (
        <>
          <label className="mt-3 block text-xs text-muted" htmlFor="translation-review-selection">
            Готовый перевод
          </label>
          <select
            id="translation-review-selection"
            className="mt-1 w-full rounded-sm border border-border bg-canvas px-2 py-1.5 text-xs text-text"
            value={review.selectedTranslationId ?? ''}
            onChange={(event) => review.select(event.target.value)}
          >
            {readyLinks.map((link) => (
              <option key={link.translationId} value={link.translationId}>
                {link.sourceFormat.toUpperCase()} · {link.sourceLanguage.toUpperCase()} → RU ·{' '}
                {link.translationId.slice(0, 8)}
              </option>
            ))}
          </select>
          <TranslationHistorySelector
            entries={history.entries}
            previewResultId={review.previewResultId}
            attachedResultId={
              history.observed?.selectedResultId ?? review.selected?.selectedResultId ?? null
            }
            loading={history.loading}
            selecting={history.selecting}
            hasMore={Boolean(history.nextCursor)}
            error={history.error}
            onPreview={review.preview}
            onAttach={(resultId) => void history.attach(resultId)}
            onMore={() => void history.more()}
          />
          {review.loadingPage && (
            <p role="status" className="mt-3 text-xs text-muted">
              Загружаем сравнение…
            </p>
          )}
          {page && (
            <>
              <p className="mt-3 text-xs text-muted">
                {page.needsReview ? 'Требует проверки' : 'Проверено'} · {page.totalSegments} реплик
              </p>
              {!canEdit && (
                <p className="mt-2 text-xs text-muted">Загружаем проверенный перевод для правки.</p>
              )}
              <div className="mt-2 space-y-2">
                {page.segments.map((segment) => (
                  <div
                    key={segment.id}
                    className="grid gap-2 rounded-sm border border-border p-2 text-xs sm:grid-cols-2"
                  >
                    <div>
                      <p className="mb-1 text-subtle">
                        {segment.ordinal + 1} ·{' '}
                        {formatSubtitleRange(segment.startMs, segment.endMs)} · Оригинал
                      </p>
                      {segment.sourceLines.map((line, index) => (
                        <p key={index} className="whitespace-pre-wrap break-words text-text">
                          {line}
                        </p>
                      ))}
                    </div>
                    <div>
                      <p className="mb-1 text-subtle">Перевод</p>
                      {segment.translatedLines.map((line, index) => (
                        <p key={index} className="whitespace-pre-wrap break-words text-text">
                          {line}
                        </p>
                      ))}
                      {projectId && review.selected && canEdit && (
                        <TranslationSegmentEditor
                          key={page.resultId}
                          projectId={projectId}
                          translationId={review.selected.translationId}
                          baseResultId={page.resultId}
                          segment={segment}
                          onPublished={(record) => {
                            setPublication({ projectId, record });
                            review.refresh();
                          }}
                        />
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-3 flex items-center gap-2">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={review.previous}
                  disabled={review.offset === 0 || review.loadingPage}
                >
                  Назад
                </Button>
                <span className="text-xs text-muted">
                  {Math.floor(review.offset / COMPARISON_PAGE_SIZE) + 1}
                </span>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={review.next}
                  disabled={
                    review.offset + page.segments.length >= page.totalSegments || review.loadingPage
                  }
                >
                  Далее
                </Button>
              </div>
            </>
          )}
        </>
      )}
    </section>
  );
}
