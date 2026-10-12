import type { SpeechRunView } from '@/shared/api/contracts/speech';
type Props = {
  run: SpeechRunView | undefined;
  busy: boolean;
  active: boolean | null | undefined;
  audio: string | null;
  start: (resume: boolean) => Promise<void>;
  cancel: () => Promise<void>;
  play: () => Promise<void>;
};
const button = 'rounded border border-border px-3 py-2 text-xs text-text disabled:opacity-40';
export function SpeechRunControls({ run, busy, active, audio, start, cancel, play }: Props) {
  return (
    <>
      {run && (
        <div className="space-y-2 text-muted">
          <p>
            {run.utterance_count} реплик · {run.chunk_count} частей
            {run.job ? ` · ${run.job.progress.percent}% · ${run.job.progress.message}` : ''}
          </p>
          {run.job?.error && <p role="alert">{run.job.error}</p>}
          <div className="flex flex-wrap gap-2">
            {!run.job && (
              <button className={button} disabled={busy} onClick={() => void start(false)}>
                Озвучить
              </button>
            )}
            {run.job && ['failed', 'cancelled'].includes(run.job.status) && !run.result && (
              <button className={button} disabled={busy} onClick={() => void start(true)}>
                Продолжить
              </button>
            )}
            {active && (
              <button
                className={button}
                disabled={busy || run.job?.status === 'cancelling'}
                onClick={() => void cancel()}
              >
                Отменить
              </button>
            )}
            {run.result && (
              <button className={button} disabled={busy} onClick={() => void play()}>
                Прослушать WAV
              </button>
            )}
          </div>
          {run.result && (
            <p>Готово · {(run.result.duration_ms / 1000).toFixed(2)} с · аудио сохранено</p>
          )}
          {audio && <audio controls autoPlay src={audio} aria-label="Готовая озвучка" />}
        </div>
      )}
    </>
  );
}
