import type { PipelineStep, View } from '@/shared/router';
export type { PipelineDisplayStatus } from '../../model/pipelineStatus';

export const locationLabel: Record<View, string> = {
  home: 'Проекты',
  project: 'Рабочее пространство',
  settings: 'Настройки',
};

export const stepLabel: Record<PipelineStep, string> = {
  source: 'Загрузка видео',
  subtitles: 'Субтитры',
};
