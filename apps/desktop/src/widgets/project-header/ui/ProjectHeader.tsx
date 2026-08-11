import { useEffect, useRef } from 'react';
import {
  PageHeader,
  PageHeaderGroup,
  PageTitle,
  PageDescription,
  PageActions,
} from '../../../shared/ui/page-layout';
import { useProjectContext } from '@/entities/project';
import { formatDuration, formatSourceLabel, getProjectStatusTone } from '@/entities/media';
import { useNavigation } from '@/shared/router';
import { Badge } from '@/shared/ui/badge';
import { Button } from '@/shared/ui/button';

export const ProjectHeader = () => {
  const { project } = useProjectContext();
  const { pipelineStep } = useNavigation();
  const titleRef = useRef<HTMLHeadingElement>(null);
  const isSourceStep = pipelineStep === 'source';
  const sourceLabel = formatSourceLabel(project?.source ?? null);

  useEffect(() => {
    if (project?.id) titleRef.current?.focus();
  }, [project?.id, pipelineStep]);

  const sourceState = getSourceState(project);

  return (
    <PageHeader className="min-w-0 items-start border-b border-border bg-bg px-4 py-4 sm:px-6">
      <PageHeaderGroup className="min-w-0">
        <PageTitle
          ref={titleRef}
          tabIndex={-1}
          className="min-w-0 truncate !text-xl !font-semibold focus:outline-none"
        >
          {isSourceStep ? 'Источник видео' : 'Субтитры'}
        </PageTitle>
        <PageDescription className="mt-0.5 !text-[13px]">
          {isSourceStep
            ? 'YouTube-ссылка или локальный MP4-файл'
            : 'Получите исходные субтитры для перевода'}
        </PageDescription>

        {isSourceStep && project && (
          <div className="mt-3 flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[11px] text-subtle">
            <span>Запуск: {formatTime(project.createdAt)}</span>
            {project.metadata && (
              <span>Длительность: {formatDuration(project.metadata.durationMs)}</span>
            )}
            {project.source && (
              <span className="min-w-0 truncate text-info" title={sourceLabel}>
                → {sourceLabel}
              </span>
            )}
          </div>
        )}
      </PageHeaderGroup>

      <PageActions className="flex-row items-center">
        {isSourceStep ? (
          <>
            <Badge variant={sourceState.tone} size="sm" className="shrink-0">
              <span className="mr-1.5 inline-block h-1.5 w-1.5 rounded-full bg-current" />
              {sourceState.label}
            </Badge>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              disabled
              title="Замена источника для существующего проекта пока не поддерживается"
              style={{ opacity: 1 }}
            >
              Перезапустить
            </Button>
          </>
        ) : project ? (
          <Badge variant={sourceState.tone} size="sm" className="shrink-0">
            <span className="mr-1.5 inline-block h-1.5 w-1.5 rounded-full bg-current" />
            {project.status === 'processing' ? 'Выполняется' : sourceState.label}
          </Badge>
        ) : null}
      </PageActions>
    </PageHeader>
  );
};

function getSourceState(project: ReturnType<typeof useProjectContext>['project']): {
  label: string;
  tone: ReturnType<typeof getProjectStatusTone>;
} {
  if (!project) return { label: 'Загрузка', tone: 'muted' };
  if (project.status === 'failed') return { label: 'Ошибка', tone: 'danger' };
  if (project.source && project.metadata) return { label: 'Готово', tone: 'success' };
  if (project.status === 'processing') return { label: 'Выполняется', tone: 'primary' };
  return { label: 'Ожидание', tone: 'muted' };
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '--:--:--';
  return date.toLocaleTimeString('ru-RU', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}
