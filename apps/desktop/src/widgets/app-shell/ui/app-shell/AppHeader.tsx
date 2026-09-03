import type { RefObject } from 'react';
import type { PipelineStep, View } from '@/shared/router';
import { Button } from '@/shared/ui/button';
import { Icon } from '@/shared/ui/icon';
import { locationLabel, stepLabel } from './model';

type Props = {
  currentView: View;
  pipelineStep: PipelineStep;
  projectTitle: string;
  hasProject: boolean;
  hasJobContext: boolean;
  activeJobs: number;
  queueOpen: boolean;
  queueButtonRef: RefObject<HTMLButtonElement | null>;
  onHome: () => void;
  onBackFromSettings: () => void;
  onToggleQueue: () => void;
};

export function AppHeader(props: Props) {
  return (
    <header className="flex h-9 shrink-0 items-center justify-between border-b border-border bg-surface px-3">
      <div className="flex min-w-0 items-center gap-1.5">
        <div className="flex h-6 w-6 items-center justify-center rounded-xs bg-primary text-xs font-bold text-primary-foreground lg:hidden">
          A
        </div>
        {props.currentView === 'project' && (
          <>
            <button
              type="button"
              onClick={props.onHome}
              aria-label="Back to projects"
              className="rounded-xs p-1 text-subtle transition-colors hover:bg-surface-raised hover:text-muted"
            >
              <Icon name="ChevronLeft" size={14} />
            </button>
            <button
              type="button"
              disabled
              aria-label="Forward unavailable"
              className="rounded-xs p-1 text-subtle opacity-35"
            >
              <Icon name="ChevronRight" size={14} />
            </button>
            <span className="mx-1 h-4 w-px bg-border" aria-hidden="true" />
          </>
        )}
        {props.currentView === 'settings' && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={props.onBackFromSettings}
            leftIcon={<Icon name="ArrowLeft" size={14} />}
            className="!px-2"
            aria-label="Back"
          >
            Назад
          </Button>
        )}
        <button
          type="button"
          onClick={props.onHome}
          aria-label="Project list breadcrumb"
          className="rounded-xs px-1 py-0.5 text-xs text-subtle transition-colors hover:bg-surface-raised hover:text-muted"
        >
          Проекты
        </button>
        {props.currentView === 'project' && (
          <>
            <span className="text-border-strong">/</span>
            <span className="max-w-48 truncate px-1 text-xs font-medium text-muted">
              {props.projectTitle}
            </span>
            <span className="text-border-strong">/</span>
            <span className="truncate px-1 text-xs text-subtle">
              {stepLabel[props.pipelineStep]}
            </span>
          </>
        )}
        {props.currentView === 'settings' && (
          <>
            <span className="text-border-strong">/</span>
            <span className="truncate px-1 text-xs font-medium text-muted">
              {locationLabel.settings}
            </span>
          </>
        )}
      </div>
      {props.hasJobContext && (
        <button
          ref={props.queueButtonRef}
          type="button"
          aria-expanded={props.queueOpen}
          aria-controls="global-job-queue"
          onClick={props.onToggleQueue}
          className="relative flex items-center gap-1.5 rounded-sm px-3 py-1 text-xs text-subtle transition-colors hover:bg-surface-raised hover:text-muted"
        >
          <Icon name="ListFilter" size={14} />
          <span className="hidden sm:inline">Очередь</span>
          {props.activeJobs > 0 && (
            <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[9px] font-bold text-primary-foreground">
              {props.activeJobs}
            </span>
          )}
        </button>
      )}
    </header>
  );
}
