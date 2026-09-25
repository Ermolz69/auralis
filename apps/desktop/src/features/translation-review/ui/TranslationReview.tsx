import { useProjectContext } from '@/entities/project';
import { Button } from '@/shared/ui/button';
import { formatSubtitleRange } from '@/shared/lib/formatSubtitleRange';
import { useTranslationReview, COMPARISON_PAGE_SIZE } from '../model/useTranslationReview';
import { TranslationSegmentEditor } from './TranslationSegmentEditor';

export function TranslationReview({ refreshRevision = 0 }: { refreshRevision?: number }) {
  const { project } = useProjectContext();
  const review = useTranslationReview(project?.id ?? null, refreshRevision);
  const page = review.page?.resultId === review.selected?.selectedResultId ? review.page : null;
  const readyLinks = review.links.filter((link) => link.selectedResultId);

  return (
    <section className="rounded-md border border-border bg-surface p-4" aria-label="Переводы проекта">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold text-text">Переводы проекта</h2>
        <Button variant="ghost" size="sm" onClick={review.refresh} loading={review.loadingLinks}>
          Обновить переводы
        </Button>
      </div>
      {review.error && <p role="alert" className="mt-3 text-xs text-danger">{review.error}</p>}
      {!review.loadingLinks && review.links.length === 0 && (
        <p className="mt-3 text-xs text-muted">Пока нет переводов для этого проекта.</p>
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
                {link.sourceFormat.toUpperCase()} · {link.sourceLanguage.toUpperCase()} → RU · {link.translationId.slice(0, 8)}
              </option>
            ))}
          </select>
          {review.loadingPage && <p role="status" className="mt-3 text-xs text-muted">Загружаем сравнение…</p>}
          {page && (
            <>
              <p className="mt-3 text-xs text-muted">
                {page.needsReview ? 'Требует проверки' : 'Проверено'} · {page.totalSegments} реплик
              </p>
              <div className="mt-2 space-y-2">
                {page.segments.map((segment) => (
                  <div key={segment.id} className="grid gap-2 rounded-sm border border-border p-2 text-xs sm:grid-cols-2">
                    <div>
                      <p className="mb-1 text-subtle">{segment.ordinal + 1} · {formatSubtitleRange(segment.startMs, segment.endMs)} · Оригинал</p>
                      {segment.sourceLines.map((line, index) => <p key={index} className="whitespace-pre-wrap break-words text-text">{line}</p>)}
                    </div>
                    <div>
                      <p className="mb-1 text-subtle">Перевод</p>
                      {segment.translatedLines.map((line, index) => <p key={index} className="whitespace-pre-wrap break-words text-text">{line}</p>)}
                      {project && review.selected && (
                        <TranslationSegmentEditor
                          key={page.resultId}
                          projectId={project.id}
                          translationId={review.selected.translationId}
                          baseResultId={page.resultId}
                          segment={segment}
                          onPublished={() => review.refresh()}
                        />
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-3 flex items-center gap-2">
                <Button variant="secondary" size="sm" onClick={review.previous} disabled={review.offset === 0 || review.loadingPage}>Назад</Button>
                <span className="text-xs text-muted">{Math.floor(review.offset / COMPARISON_PAGE_SIZE) + 1}</span>
                <Button variant="secondary" size="sm" onClick={review.next} disabled={review.offset + page.segments.length >= page.totalSegments || review.loadingPage}>Далее</Button>
              </div>
            </>
          )}
        </>
      )}
    </section>
  );
}
