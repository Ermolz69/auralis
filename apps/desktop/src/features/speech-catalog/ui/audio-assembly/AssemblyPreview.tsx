import type { AudioAssemblyPreview as Preview } from '@/shared/api/contracts/audioAssembly';
import { PagedPreview } from '@/shared/ui/paged-preview/PagedPreview';
import { conflictText } from '../../model/audio-assembly/conflictText';

type Props = {
  preview: Preview;
  current: boolean;
  busy: boolean;
  mode: 'continuous' | 'subtitle_window';
  pauses: Record<number, number>;
  setPause: (index: number, value: number) => void;
};
export function AssemblyPreview({ preview, current, busy, mode, pauses, setPause }: Props) {
  return (
    <div className="space-y-2">
      <p>
        {preview.utterances.length} полных реплик · 48 кГц ·{' '}
        {preview.channels === 1 ? 'моно' : 'стерео'}
        {preview.frames !== null && current
          ? ` · итог ${(preview.frames / 48000).toFixed(2)} с`
          : ''}
      </p>
      {!current && (
        <p role="status">Настройки изменены. Измерьте и проверьте новый предпросмотр.</p>
      )}
      {current && preview.conflicts.length > 0 && (
        <ul role="alert" className="space-y-1">
          {preview.conflicts.map((item, index) => (
            <li key={index}>{conflictText(item)}</li>
          ))}
        </ul>
      )}
      {!preview.renderer_available && (
        <p role="alert">
          {preview.renderer_error ?? 'Восстановите управляемый FFmpeg в установке Auralis.'}
        </p>
      )}
      <PagedPreview
        label="Тайминг реплик"
        items={preview.utterances}
        renderItem={(item, index) => (
          <div
            key={item.utterance_id}
            className="flex flex-wrap items-center gap-2 rounded border border-border p-2"
          >
            <span>
              {index + 1}. {item.utterance_id} · речь {(item.frame_count / 48).toFixed(1)} мс ·{' '}
              {item.source_window
                ? `окно ${item.source_window.start_ms}–${item.source_window.end_ms} мс`
                : 'без исходного окна'}
            </span>
            <label>
              Пауза после, мс
              <input
                aria-label={`Пауза после ${item.utterance_id}, мс`}
                className="ml-1 w-24 rounded border border-border bg-surface p-1 text-text"
                type="number"
                min="0"
                step="1"
                disabled={busy || mode !== 'continuous'}
                value={pauses[index] ?? 0}
                onChange={(event) => setPause(index, Number(event.target.value))}
              />
            </label>
          </div>
        )}
      />
    </div>
  );
}
