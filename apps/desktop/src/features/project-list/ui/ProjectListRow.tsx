import type { RefCallback } from 'react';
import type { Project } from '@/entities/project';
import {
  formatProjectStatus,
  formatProjectTitle,
  formatSourceLabel,
  getProjectStatusTone,
} from '@/entities/media';
import { Button } from '@/shared/ui/button';
import { Card } from '@/shared/ui/card';
import { Icon } from '@/shared/ui/icon';

type ProjectListRowProps = {
  project: Project;
  isDeleting: boolean;
  isAnyDeleting: boolean;
  openButtonRef: RefCallback<HTMLButtonElement>;
  deleteButtonRef: RefCallback<HTMLButtonElement>;
  onOpen: (project: Project) => void;
  onDelete: (project: Project) => void;
};

export const ProjectListRow = ({
  project,
  isDeleting,
  isAnyDeleting,
  openButtonRef,
  deleteButtonRef,
  onOpen,
  onDelete,
}: ProjectListRowProps) => {
  const displayTitle = formatProjectTitle(project.title, project.source);
  const sourceLabel = formatSourceLabel(project.source);
  const statusLabel = formatProjectStatus(project.status);
  const statusTone = getProjectStatusTone(project.status);
  const statusClass = {
    success: 'bg-success',
    danger: 'bg-danger',
    primary: 'bg-primary animate-pulse',
    warning: 'bg-warning',
    muted: 'bg-muted',
  }[statusTone];

  return (
    <Card
      className={`group relative overflow-hidden p-0 transition-colors flex items-center justify-between shadow-sm border border-secondary min-w-0 ${isDeleting ? 'opacity-50' : 'hover:bg-bg/50'}`}
      aria-busy={isDeleting}
    >
      <button
        type="button"
        ref={openButtonRef}
        className="flex min-w-0 flex-1 items-center gap-3 p-4 text-left w-full h-full focus:outline-none focus:bg-bg/50 focus-visible:ring-2 focus-visible:ring-focus"
        onClick={() => onOpen(project)}
        disabled={isAnyDeleting}
        aria-label={`Open ${displayTitle}. Status: ${statusLabel}. Source: ${sourceLabel}`}
      >
        <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary shrink-0">
          <Icon
            name={
              project.source?.kind === 'youtubeUrl' || project.source?.kind === 'remoteUrl'
                ? 'Video'
                : 'Film'
            }
            size="md"
          />
        </div>
        <div className="flex min-w-0 flex-col text-left flex-1">
          <span className="text-text font-medium truncate max-w-full">{displayTitle}</span>
          <span className="text-muted text-xs flex items-center gap-1.5 mt-0.5">
            <span className={`w-2 h-2 rounded-full ${statusClass}`} aria-hidden="true"></span>
            <span className="sr-only">Status: </span>
            {statusLabel}
          </span>
          <span
            className="text-muted text-xs truncate max-w-full"
            aria-label={`Source: ${sourceLabel}`}
          >
            <span className="sr-only">Source: </span>
            {sourceLabel}
          </span>
        </div>
        <div className="shrink-0 text-muted text-xs pr-4">
          {new Date(project.updatedAt).toLocaleDateString()}
        </div>
      </button>

      <div className="pr-4 shrink-0 flex items-center">
        <Button
          ref={deleteButtonRef}
          variant="ghost"
          size="sm"
          className="opacity-0 focus:opacity-100 group-focus-within:opacity-100 group-hover:opacity-100 transition-opacity"
          loading={isDeleting}
          disabled={isAnyDeleting}
          onClick={() => onDelete(project)}
          title="Delete Project"
          aria-label={`Delete ${displayTitle}`}
          leftIcon={!isDeleting ? <Icon name="Trash2" size="sm" /> : undefined}
        />
      </div>
    </Card>
  );
};
