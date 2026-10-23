import { isActiveJobStatus, type JobDto } from '@/entities/job';
import { Progress } from '@/shared/ui/progress';
const labels = {
  pending: 'В очереди',
  running: 'Выполняется',
  cancelling: 'Останавливается',
  completed: 'Готово',
  failed: 'Ошибка',
  cancelled: 'Отменено',
};
export function ProcessQueueStrip({ jobs }: { jobs: JobDto[] }) {
  const active = jobs.filter((job) => isActiveJobStatus(job.status));
  return (
    <section
      aria-label="Текущие процессы"
      className="flex min-w-0 shrink-0 items-center gap-3 border-t border-border bg-surface px-3 py-3"
    >
      <h2 className="shrink-0 text-xs font-medium text-muted">
        Процессы <span aria-label="Количество текущих процессов">{active.length}</span>
      </h2>
      {active.length ? (
        <ul
          aria-label="Очередь процессов"
          className="flex min-w-0 flex-1 gap-2 overflow-x-auto p-0.5"
        >
          {active.map((job) => (
            <li
              key={job.id}
              className="flex w-64 shrink-0 flex-col gap-1 rounded-[6px] border border-border bg-bg px-3 py-2"
              title={job.progress.message || job.title}
            >
              <div className="flex items-center justify-between gap-3 text-xs">
                <span className="truncate font-medium">{job.title}</span>
                <span className="shrink-0 tabular-nums">{Math.round(job.progress.percent)}%</span>
              </div>
              <span className="text-xs text-muted">{labels[job.status]}</span>
              <Progress aria-label={`Прогресс: ${job.title}`} value={job.progress.percent} />
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-xs text-muted">Нет активных процессов</p>
      )}
    </section>
  );
}
