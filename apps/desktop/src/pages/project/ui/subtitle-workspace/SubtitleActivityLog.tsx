import type { Job } from '@/entities/job';
import { formatJobStatus, formatTime } from './model';
import { WorkspaceSection } from './WorkspaceSection';

export function SubtitleActivityLog({ jobs }: { jobs: Job[] }) {
  return (
    <WorkspaceSection title="Лог">
      <div className="min-h-[76px] rounded-md border border-border bg-canvas px-3 py-2.5 font-mono text-[11px] leading-5">
        {jobs.length === 0 ? (
          <p className="text-subtle">Импорт субтитров ещё не запускался</p>
        ) : (
          jobs.map((job) => (
            <div key={job.id} className="flex min-w-0 gap-3">
              <time className="shrink-0 text-subtle" dateTime={job.updatedAt}>
                {formatTime(job.updatedAt)}
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
  );
}
