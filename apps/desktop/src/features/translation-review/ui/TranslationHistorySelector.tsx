import type { TranslationHistoryEntry } from '@/shared/api/contracts/translation';
import { Button } from '@/shared/ui/button';

type Props = {
  entries: TranslationHistoryEntry[];
  previewResultId: string | null;
  attachedResultId: string | null;
  loading: boolean;
  selecting: boolean;
  hasMore: boolean;
  error: string | null;
  onPreview: (resultId: string) => void;
  onAttach: (resultId: string) => void;
  onMore: () => void;
};

export function TranslationHistorySelector(props: Props) {
  return (
    <div className="mt-3 space-y-2">
      <label className="block text-xs text-muted" htmlFor="translation-history-selection">
        Версия для сравнения
      </label>
      <select
        id="translation-history-selection"
        className="w-full rounded-sm border border-border bg-canvas px-2 py-1.5 text-xs text-text"
        value={props.previewResultId ?? ''}
        onChange={(event) => props.onPreview(event.target.value)}
        disabled={props.loading || props.selecting}
      >
        {props.previewResultId &&
          !props.entries.some((entry) => entry.resultId === props.previewResultId) && (
            <option value={props.previewResultId}>
              {props.previewResultId === props.attachedResultId
                ? 'Прикреплённая версия'
                : 'Просматриваемая версия'}
            </option>
          )}
        {props.entries.map((entry) => (
          <option key={entry.resultId} value={entry.resultId}>
            Запуск {entry.runId.slice(0, 8)} · версия {entry.resultRevision}
            {entry.resultId === props.attachedResultId ? ' · Прикреплена' : ''}
            {entry.needsReview ? ' · Требует проверки' : ''}
          </option>
        ))}
      </select>
      {props.hasMore && (
        <Button
          size="sm"
          variant="ghost"
          onClick={props.onMore}
          loading={props.loading}
          disabled={props.selecting}
        >
          Ещё версии
        </Button>
      )}
      {props.previewResultId && props.previewResultId !== props.attachedResultId && (
        <Button
          size="sm"
          variant="secondary"
          onClick={() => props.onAttach(props.previewResultId!)}
          loading={props.selecting}
          disabled={props.loading}
        >
          Использовать эту версию
        </Button>
      )}
      {props.error && (
        <p role="alert" className="text-xs text-danger">
          {props.error}
        </p>
      )}
    </div>
  );
}
