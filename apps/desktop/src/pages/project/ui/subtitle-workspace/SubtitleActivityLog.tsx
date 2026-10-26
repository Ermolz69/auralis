import type { Job } from '@/entities/job';
import { formatClockTime } from '@/shared/lib';
import { formatJobStatus } from './model';
import { WorkspaceSection } from '../WorkspaceSection';

export function SubtitleActivityLog({ jobs }: { jobs: Job[] }) {
  const latest = jobs.filter((job) => job.kind === 'dubbing').at(-1);
  const states = {
    pending: 'в очереди',
    running: 'выполняется',
    cancelling: 'останавливается',
    completed: 'готово',
    failed: 'ошибка',
    cancelled: 'отменено',
  };
  return (
    <section aria-label="История импорта субтитров">
      <WorkspaceSection title="Лог">
        {latest && (
          <p role="status" aria-label="Состояние импорта субтитров">
            Задание {latest.id} · {states[latest.status]}
          </p>
        )}
        {latest?.error && <p role="alert">{latest.error}</p>}
        <div className="min-h-[76px] rounded-md border border-border bg-canvas px-3 py-2.5 font-mono text-[11px] leading-5">
          {jobs.length === 0 ? (
            <p className="text-subtle">Импорт субтитров ещё не запускался</p>
          ) : (
            jobs.map((job) => (
              <div key={job.id} className="flex min-w-0 gap-3">
                <time className="shrink-0 text-subtle" dateTime={job.updatedAt}>
                  {formatClockTime(job.updatedAt)}
                </time>
                <span
                  className={
                    job.status === 'failed'
                      ? 'min-w-0 break-words text-danger'
                      : 'min-w-0 break-words text-muted'
                  }
                >
                  {job.status === 'failed'
                    ? job.error || 'Ошибка импорта субтитров'
                    : job.progress.message || formatJobStatus(job.status)}
                </span>
              </div>
            ))
          )}
        </div>
      </WorkspaceSection>
    </section>
  );
}
