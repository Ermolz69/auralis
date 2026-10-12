import { useSpeechWorkflow, type SpeechWorkflowOptions } from '../model/useSpeechWorkflow';
import { SpeechSourcePreview } from './SpeechSourcePreview';
import { SpeechRunControls } from './SpeechRunControls';
const button = 'rounded border border-border px-3 py-2 text-xs text-text disabled:opacity-40';
export function SpeechWorkflow(options: SpeechWorkflowOptions) {
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
  } = useSpeechWorkflow(options);
  return (
    <div className="mt-4 space-y-3 border-t border-border pt-3 text-xs">
      <div role="group" aria-label="Источник озвучки" className="flex gap-2">
        <button
          className={button}
          aria-pressed={mode === 'subtitles'}
          onClick={() => {
            changed();
            setMode('subtitles');
          }}
        >
          Субтитры
        </button>
        <button
          className={button}
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
              setSubtitleId(e.target.value.startsWith('artifact:') ? e.target.value.slice(9) : '');
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
        {mode === 'subtitles' && (
          <button className={button} disabled={busy} onClick={() => void refreshSources()}>
            Обновить источники
          </button>
        )}
        <button className={button} disabled={busy} onClick={() => void load()}>
          Показать текст
        </button>
        <button
          className={button}
          disabled={busy || (mode === 'text' && !text.trim())}
          onClick={() => void confirm()}
        >
          {translationId && mode === 'subtitles'
            ? 'Использовать одобренный перевод'
            : 'Текст проверен'}
        </button>
        <button
          className={button}
          disabled={busy || !review || !profileId || !voiceId || !referenceId}
          onClick={() => void prepare()}
        >
          Подготовить план
        </button>
      </div>
      {review && (
        <p role="status" className="text-muted">
          Снимок проверенного текста сохранён.
        </p>
      )}
      <SpeechSourcePreview
        source={source}
        voices={options.voices}
        assignments={assignments}
        assign={assign}
      />
      {error && (
        <p role="alert" className="text-text">
          {error}
        </p>
      )}
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
            {runs.map((item) => (
              <option key={item.run_id} value={item.run_id}>
                {item.profile_id} · {item.run_id.slice(0, 8)} · {item.job?.status ?? 'план готов'}
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
  );
}
