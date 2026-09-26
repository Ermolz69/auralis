import type {
  ComparedTranslationSegment,
  TranslationPublication,
} from '@/shared/api/contracts/translation';
import { Button } from '@/shared/ui/button';
import { useHistoricalTranslationEdit } from '../model/useHistoricalTranslationEdit';

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
  const edit = useHistoricalTranslationEdit(
    projectId,
    translationId,
    baseResultId,
    segment,
    onPublished,
  );
  if (!edit.open)
    return (
      <Button variant="ghost" size="sm" onClick={() => edit.setOpen(true)}>
        Исправить перевод
      </Button>
    );

  return (
    <div className="mt-2 space-y-2 border-t border-border pt-2">
      <p className="text-xs text-muted">Новая версия сохранит остальные реплики из этой версии.</p>
      {edit.preparing && (
        <p role="status" className="text-xs text-muted">
          Проверяем версию для правки…
        </p>
      )}
      {edit.lines.map((line, index) => (
        <label key={index} className="block text-xs text-muted">
          Строка перевода {index + 1}
          <input
            type="text"
            value={line}
            onChange={(event) =>
              edit.setLines((current) =>
                current.map((value, item) => (item === index ? event.target.value : value)),
              )
            }
            disabled={Boolean(edit.attempt) || edit.saving || Boolean(edit.publication)}
            className="mt-1 w-full rounded-sm border border-border bg-canvas px-2 py-1.5 text-xs text-text"
          />
        </label>
      ))}
      {edit.error && (
        <p role="alert" className="text-xs text-danger">
          {edit.error.message}
        </p>
      )}
      {edit.publication && (
        <p role="status" className="text-xs text-muted">
          Правка сохранена отдельной версией. Готовность и выбор показаны выше.
        </p>
      )}
      {!edit.publication && (
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            onClick={() => void edit.save()}
            loading={edit.saving}
            disabled={!edit.context || edit.preparing || edit.lines.some((line) => !line.trim())}
          >
            {edit.attempt ? 'Повторить сохранение' : 'Сохранить новую версию'}
          </Button>
          {!edit.attempt && (
            <Button variant="ghost" size="sm" onClick={() => edit.setOpen(false)}>
              Отмена
            </Button>
          )}
          {edit.error &&
            !edit.saving &&
            (!edit.attempt || (!edit.attempt.edited && edit.error.code === 'CONFLICT')) && (
              <Button variant="ghost" size="sm" onClick={edit.fresh}>
                Обновить версию для правки
              </Button>
            )}
        </div>
      )}
    </div>
  );
}
