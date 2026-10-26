import type { Job } from '@/shared/api/contracts/job';

const labels = {
  pending: 'в очереди',
  running: 'выполняется',
  cancelling: 'останавливается',
  completed: 'готово',
  failed: 'ошибка',
  cancelled: 'отменено',
};

export function JobState({ job, label }: { job: Job | null; label: string }) {
  return (
    <p role="status" aria-label={label}>
      {job ? `Задание ${job.id} · ${labels[job.status]}` : 'План готов'}
    </p>
  );
}
