import type { JobStatus } from '@/entities/job';
import type { PipelineStep, View } from '@/shared/router';

export const locationLabel: Record<View, string> = {
  home: 'Проекты',
  project: 'Рабочее пространство',
  settings: 'Настройки',
};

export const stepLabel: Record<PipelineStep, string> = {
  source: 'Загрузка видео',
  subtitles: 'Субтитры',
};

export type PipelineDisplayStatus = JobStatus | 'idle' | 'unavailable';

export function getPipelineStatus(hasSource: boolean, statuses: JobStatus[]) {
  const active = statuses.find((status) => status === 'running' || status === 'pending');
  const failed = statuses.includes('failed');
  const completed = statuses.includes('completed');
  return {
    source: hasSource ? ('completed' as const) : ('idle' as const),
    subtitles:
      active ??
      (failed ? ('failed' as const) : completed ? ('completed' as const) : ('idle' as const)),
  };
}
