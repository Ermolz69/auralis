import type { MarkerPreparation } from '@/shared/api/contracts/speech';
export function MarkerPreview({
  preparation,
  applied,
  apply,
  busy,
}: {
  preparation: MarkerPreparation;
  applied: boolean;
  apply: () => void;
  busy: boolean;
}) {
  return (
    <div className="space-y-2 rounded border border-border p-3 text-xs text-muted">
      <p>
        Подготовка текста: {preparation.changed_targets} правок · {preparation.unresolved_targets}{' '}
        неоднозначных мест
      </p>
      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <h3 className="mb-1 font-medium text-text">Исходный текст</h3>
          <pre className="max-h-48 overflow-auto whitespace-pre-wrap">
            {preparation.original.text}
          </pre>
        </div>
        <div>
          <h3 className="mb-1 font-medium text-text">Предлагаемый вариант</h3>
          <pre className="max-h-48 overflow-auto whitespace-pre-wrap">
            {preparation.prepared.text}
          </pre>
        </div>
      </div>
      <p>Проверьте числа, сокращения и смысл. Подготовка не заменяет проверку текста.</p>
      {applied ? (
        <p role="status">Подготовленный вариант выбран. Подтвердите проверку текста.</p>
      ) : (
        <button
          className="rounded border border-border px-3 py-2 text-text"
          disabled={busy}
          onClick={apply}
        >
          Использовать подготовленный текст
        </button>
      )}
    </div>
  );
}
