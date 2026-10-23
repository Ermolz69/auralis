import { useSpeechWorkflow, type SpeechWorkflowOptions } from '../model/useSpeechWorkflow';
import { SpeechSourcePreview } from './SpeechSourcePreview';
import { SpeechRunControls } from './SpeechRunControls';
import type { ReactNode } from 'react';
import { MarkerPreview } from './MarkerPreview';
const button = 'rounded border border-border px-3 py-2 text-xs text-text disabled:opacity-40';
const runStatus = {
  pending: 'в очереди',
  running: 'озвучивается',
  cancelling: 'останавливается',
  completed: 'готово',
  failed: 'ошибка',
  cancelled: 'отменён',
};
export function SpeechWorkflow({
  renderVoiceSettings,
  stage,
  ...options
}: SpeechWorkflowOptions & { renderVoiceSettings?: (disabled: boolean) => ReactNode }) {
  const { profileId, voiceId, referenceId } = options;
  const {
    mode,
    setMode,
    text,
    setText,
    translationId,
    setTranslationId,
    subtitleId,
    setSubtitleId,
    subtitleFiles,
    translations,
    source,
    review,
    assignments,
    runs,
    selected,
    setSelected,
    error,
    busy,
    audio,
    run,
    active,
    changed,
    refreshSources,
    load,
    confirm,
    prepare,
    start,
    play,
    assign,
    cancel,
    markerPreparation,
    markerApplied,
    normalize,
    applyPreparation,
  } = useSpeechWorkflow(options);
  return (
    <div className="mt-4 space-y-3 border-t border-border pt-3 text-xs">
      <div hidden={stage === 'voices'} className="space-y-3">
        <h3 className="text-sm font-medium text-text">1. Текст и подготовка</h3>
        <div role="group" aria-label="Источник озвучки" className="flex gap-2">
          <button
            className={`${button} ${mode === 'subtitles' ? 'bg-surface-raised border-primary' : ''}`}
            aria-pressed={mode === 'subtitles'}
            onClick={() => {
              changed();
              setMode('subtitles');
            }}
          >
            Субтитры
          </button>
          <button
            className={`${button} ${mode === 'text' ? 'bg-surface-raised border-primary' : ''}`}
            aria-pressed={mode === 'text'}
            onClick={() => {
              changed();
              setMode('text');
            }}
          >
            Текст
          </button>
        </div>
        {mode === 'text' ? (
          <textarea
            aria-label="Русский текст для озвучки"
            className="min-h-28 w-full rounded border border-border bg-surface p-2 text-text"
            value={text}
            onChange={(e) => {
              changed();
              setText(e.target.value);
            }}
          />
        ) : (
          <label className="block text-muted">
            Источник субтитров
            <select
              aria-label="Источник субтитров"
              className="ml-2 rounded border border-border bg-surface p-2 text-text"
              value={
                translationId
                  ? `translation:${translationId}`
                  : subtitleId
                    ? `artifact:${subtitleId}`
                    : ''
              }
              onChange={(e) => {
                changed();
                setTranslationId(
                  e.target.value.startsWith('translation:') ? e.target.value.slice(12) : '',
                );
                setSubtitleId(
                  e.target.value.startsWith('artifact:') ? e.target.value.slice(9) : '',
                );
              }}
            >
              <option value="">Исходные русские субтитры проекта</option>
              {subtitleFiles.map((item) => (
                <option key={item.id} value={`artifact:${item.id}`}>
                  Импортированные русские субтитры · {item.id.slice(0, 8)}
                </option>
              ))}
              {translations
                .filter((item) => item.selectedResultId)
                .map((item) => (
                  <option key={item.translationId} value={`translation:${item.translationId}`}>
                    Выбранный перевод · {item.translationId.slice(0, 8)}
                  </option>
                ))}
            </select>
          </label>
        )}
        <div className="flex flex-wrap gap-2">
          <button
            className={button}
            hidden={stage === 'speech'}
            disabled={busy || !!active || (mode === 'text' && !text.trim())}
            onClick={() => void normalize()}
          >
            Подготовить текст
          </button>
          {mode === 'subtitles' && (
            <button className={button} disabled={busy} onClick={() => void refreshSources()}>
              Обновить источники
            </button>
          )}
          {mode === 'subtitles' && (
            <button className={button} disabled={busy} onClick={() => void load()}>
              Показать текст
            </button>
          )}
          <button
            className={button}
            disabled={busy || (mode === 'text' && !text.trim())}
            onClick={() => void confirm()}
          >
            {translationId && mode === 'subtitles' && !markerApplied
              ? 'Использовать одобренный перевод'
              : 'Текст проверен'}
          </button>
        </div>
        {markerPreparation && (
          <MarkerPreview
            preparation={markerPreparation}
            applied={markerApplied}
            apply={applyPreparation}
            busy={busy || !!active}
          />
        )}
        {review && (
          <p role="status" className="text-muted">
            Снимок проверенного текста сохранён.
          </p>
        )}
        {mode === 'subtitles' && (
          <SpeechSourcePreview
            source={source}
            voices={options.voices}
            assignments={assignments}
            assign={assign}
          />
        )}
      </div>
      <div hidden={stage === 'text'}>{renderVoiceSettings?.(busy || !!active)}</div>
      {error && (
        <p role="alert" className="text-text">
          {error}
        </p>
      )}
      <div hidden={stage === 'text' || stage === 'voices'} className="space-y-3">
        <h3 className="text-sm font-medium text-text">3. Озвучка и результат</h3>
        <button
          className={button}
          disabled={busy || !review || !profileId || !voiceId || !referenceId}
          onClick={() => void prepare()}
        >
          Подготовить озвучку
        </button>
        {runs.length > 0 && (
          <label className="block text-muted">
            Сохранённые запуски
            <select
              aria-label="Сохранённые запуски"
              className="ml-2 max-w-full rounded border border-border bg-surface p-2 text-text"
              value={selected}
              onChange={(e) => setSelected(e.target.value)}
            >
              <option value="">Выберите запуск</option>
              {runs.map((item, q) => (
                <option key={item.run_id} value={item.run_id}>
                  Запуск {q + 1} · {item.profile_id.includes('espeech') ? 'ESpeech' : 'VoxCPM2'} ·{' '}
                  {item.job ? runStatus[item.job.status] : 'план готов'}
                </option>
              ))}
            </select>
          </label>
        )}
        <SpeechRunControls
          run={run}
          busy={busy}
          active={active}
          audio={audio}
          start={start}
          cancel={cancel}
          play={play}
        />
        <p className="text-muted">
          Временные окна сохраняются. Подгонка речи под видео и экспорт видео — следующий этап.
        </p>
      </div>
    </div>
  );
}
