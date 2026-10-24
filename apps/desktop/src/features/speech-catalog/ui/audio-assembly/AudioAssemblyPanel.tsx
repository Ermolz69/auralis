import type { SpeechRunView } from '@/shared/api/contracts/speech';
import { useAudioAssembly } from '../../model/audio-assembly/useAudioAssembly';
import { AssemblyPreview } from './AssemblyPreview';

const button = 'rounded border border-border px-3 py-2 text-xs text-text disabled:opacity-40';
const statuses = {
  pending: 'в очереди',
  running: 'собирается',
  cancelling: 'останавливается',
  completed: 'готово',
  failed: 'ошибка',
  cancelled: 'отменено',
};
export function AudioAssemblyPanel({
  projectId,
  source,
}: {
  projectId: string;
  source: SpeechRunView | undefined;
}) {
  const work = useAudioAssembly(projectId, source);
  return (
    <section
      aria-label="Сборка итогового аудио"
      className="space-y-3 border-t border-border pt-3 text-xs text-muted"
    >
      <h3 className="text-sm font-medium text-text">4. Итоговое аудио</h3>
      <p>Исходный синтез сохранён отдельно. Сборка использует полные реплики с исходным темпом.</p>
      {source?.result ? (
        <p>
          Источник: запуск {source.run_id} · {source.profile_id} · {source.utterance_count} реплик.
        </p>
      ) : (
        <p>Выберите готовый запуск озвучки, чтобы подготовить новый итоговый WAV.</p>
      )}
      <fieldset disabled={work.busy} className="space-y-2">
        <label className="block">
          Режим тайминга
          <select
            aria-label="Режим тайминга"
            className="ml-2 rounded border border-border bg-surface p-2 text-text"
            value={work.mode}
            onChange={(event) =>
              work.setMode(
                event.target.value === 'subtitle_window' ? 'subtitle_window' : 'continuous',
              )
            }
          >
            <option value="continuous">Непрерывное повествование с паузами</option>
            <option value="subtitle_window">Исходные окна субтитров</option>
          </select>
        </label>
        {work.mode === 'subtitle_window' && (
          <div className="space-y-1">
            <label>
              Длительность отдельной аудиолинии, мс
              <input
                aria-label="Длительность отдельной аудиолинии, мс"
                className="ml-2 w-36 rounded border border-border bg-surface p-2 text-text"
                type="number"
                min="1"
                step="1"
                value={work.duration}
                onChange={(event) => work.setDuration(event.target.value)}
              />
            </label>
            <p>
              Длительность задана вручную. Соответствие видео ещё не проверено. Тишина следует
              окнам; дополнительные паузы применяются в непрерывном режиме.
            </p>
          </div>
        )}
        <button className={button} disabled={!work.canInspect} onClick={() => void work.inspect()}>
          Измерить и показать тайминг
        </button>
      </fieldset>
      {work.preview && (
        <AssemblyPreview
          preview={work.preview}
          current={!!work.current}
          busy={work.busy}
          mode={work.mode}
          pauses={work.pauses}
          setPause={work.setPause}
        />
      )}
      {work.current && work.current.conflicts.length === 0 && (
        <label className="flex items-start gap-2">
          <input
            type="checkbox"
            checked={work.reviewed}
            disabled={work.busy}
            onChange={(event) => work.setReviewed(event.target.checked)}
          />
          Исходная озвучка, порядок реплик, окна и паузы проверены.
        </label>
      )}
      <button
        className={button}
        disabled={
          work.busy ||
          !work.current ||
          !work.reviewed ||
          !work.current.renderer_available ||
          work.current.conflicts.length > 0
        }
        onClick={() => void work.prepare()}
      >
        Подготовить сборку
      </button>
      {work.error && <p role="alert">{work.error}</p>}
      {work.history.length > 0 && (
        <label className="block">
          Сохранённые сборки
          <select
            aria-label="Сохранённые сборки"
            disabled={work.busy}
            className="ml-2 max-w-full rounded border border-border bg-surface p-2 text-text"
            value={work.selected}
            onChange={(event) => work.select(event.target.value)}
          >
            <option value="">Выберите сборку</option>
            {work.history.map((item) => (
              <option key={item.assembly_id} value={item.assembly_id}>
                {item.assembly_id.slice(0, 8)} · источник {item.source_run_id.slice(0, 8)} ·{' '}
                {item.settings.mode === 'continuous' ? 'непрерывно' : 'по окнам'} ·{' '}
                {(item.frames / 48000).toFixed(2)} с ·{' '}
                {item.job ? statuses[item.job.status] : 'план готов'}
              </option>
            ))}
          </select>
        </label>
      )}
      {work.run && (
        <div className="space-y-2">
          <p>
            Сохранённая сборка:{' '}
            {work.run.settings.mode === 'continuous'
              ? 'непрерывное повествование'
              : 'исходные окна'}{' '}
            · паузы всего {work.run.settings.pause_after_ms.reduce((sum, pause) => sum + pause, 0)}{' '}
            мс · {(work.run.frames / 48000).toFixed(2)} с.
          </p>
          <p>
            Кнопки ниже используют эту сохранённую сборку. Для новых настроек подготовьте другую
            сборку.
          </p>
          <p>
            {work.run.utterance_count} реплик · план {work.run.plan_sha256.slice(0, 12)}
            {work.run.job
              ? ` · ${work.run.job.progress.percent}% · ${work.run.job.progress.message}`
              : ''}
          </p>
          {work.run.job?.error && <p role="alert">{work.run.job.error}</p>}
          <div className="flex flex-wrap gap-2">
            {!work.run.job && (
              <button
                className={button}
                disabled={work.busy}
                onClick={() => void work.start(false)}
              >
                Собрать WAV
              </button>
            )}
            {work.run.job &&
              ['failed', 'cancelled'].includes(work.run.job.status) &&
              !work.run.result && (
                <button
                  className={button}
                  disabled={work.busy}
                  onClick={() => void work.start(true)}
                >
                  Продолжить сборку
                </button>
              )}
            {work.active && (
              <button
                className={button}
                disabled={work.busy || work.run.job?.status === 'cancelling'}
                onClick={() => void work.cancel()}
              >
                Отменить сборку
              </button>
            )}
            {work.run.result && (
              <button className={button} disabled={work.busy} onClick={() => void work.play()}>
                Прослушать итоговый WAV
              </button>
            )}
          </div>
          {work.run.result && (
            <p>
              Итоговый WAV опубликован · {(work.run.result.frames / 48000).toFixed(2)} с · исходная
              озвучка сохранена.
            </p>
          )}
        </div>
      )}
      {work.audio && <audio controls autoPlay src={work.audio} aria-label="Итоговое аудио" />}
      <p>Проверка длительности исходного видео и отдельный видеоэкспорт ещё не подключены.</p>
    </section>
  );
}
