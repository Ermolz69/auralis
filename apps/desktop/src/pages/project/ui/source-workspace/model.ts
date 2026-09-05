import type { Job } from '@/entities/job';
import type { Project } from '@/entities/project';
import type { MediaSource } from '@/shared/api/contracts/media';

export type SourceActivity = { timestamp: string; text: string; tone?: 'success' | 'danger' };

export function getSourceValue(source: MediaSource | null): string {
  if (!source) return '—';
  if (source.kind === 'youtubeUrl' || source.kind === 'remoteUrl') return source.url;
  if (source.kind === 'externalLocalFile') return source.path;
  return source.originalFilename;
}

export function buildSourceActivity(project: Project | null, jobs: Job[]): SourceActivity[] {
  if (!project) return [];
  const items: SourceActivity[] = [{ timestamp: project.createdAt, text: 'Проект создан' }];
  if (project.source)
    items.push({
      timestamp: project.updatedAt,
      text: `Источник подключён: ${getSourceValue(project.source)}`,
      tone: 'success',
    });
  if (project.metadata)
    items.push({
      timestamp: project.updatedAt,
      text: `Метаданные загружены: ${(project.metadata.container || 'media').toUpperCase()}${project.metadata.width && project.metadata.height ? ` ${project.metadata.width}×${project.metadata.height}` : ''}`,
      tone: 'success',
    });
  items.push({
    timestamp: project.updatedAt,
    text: `Статус проекта: ${projectState(project.status)}`,
    tone: project.status === 'failed' ? 'danger' : undefined,
  });
  for (const job of jobs.filter((item) => item.projectId === project.id)) {
    items.push({
      timestamp: job.updatedAt,
      text: `${job.title}: ${job.status === 'failed' ? job.error || jobState(job.status) : job.progress.message || jobState(job.status)}`,
      tone: job.status === 'failed' ? 'danger' : job.status === 'completed' ? 'success' : undefined,
    });
  }
  return items.sort((left, right) => left.timestamp.localeCompare(right.timestamp));
}

function jobState(status: string) {
  return (
    (
      {
        pending: 'ожидает запуска',
        running: 'выполняется',
        completed: 'завершено',
        failed: 'ошибка',
        cancelled: 'отменено',
      } as Record<string, string>
    )[status] ?? status
  );
}
function projectState(status: string) {
  return (
    (
      {
        draft: 'черновик',
        source_imported: 'источник импортирован',
        ready_for_processing: 'готов к обработке',
        processing: 'обработка',
        completed: 'завершён',
        failed: 'ошибка',
        cancelled: 'отменён',
      } as Record<string, string>
    )[status] ?? status
  );
}
