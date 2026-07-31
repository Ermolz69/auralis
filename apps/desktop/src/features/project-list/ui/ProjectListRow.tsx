import type { RefCallback } from 'react';
import type { Project } from '@/entities/project';
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
  const displayTitle = project.title || 'Untitled Project';
  const statusClass =
    project.status === 'completed'
      ? 'bg-success'
      : project.status === 'failed'
        ? 'bg-danger'
        : project.status === 'processing'
          ? 'bg-primary animate-pulse'
          : 'bg-muted';

  return (
    <Card
      className={`group relative overflow-hidden p-0 transition-colors flex items-center justify-between shadow-sm border border-secondary ${isDeleting ? 'opacity-50' : 'hover:bg-bg/50'}`}
      aria-busy={isDeleting}
    >
      <button
        type="button"
        ref={openButtonRef}
        className="flex-1 flex items-center gap-3 p-4 text-left w-full h-full focus:outline-none focus:bg-bg/50"
        onClick={() => onOpen(project)}
        disabled={isAnyDeleting}
        aria-label={`Open ${displayTitle}`}
      >
        <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-primary shrink-0">
          <Icon name={project.source?.kind === 'remoteUrl' ? 'Video' : 'Film'} size="md" />
        </div>
        <div className="flex flex-col text-left flex-1">
          <span className="text-text font-medium truncate max-w-[250px]" title={displayTitle}>
            {displayTitle}
          </span>
          <span className="text-muted text-xs capitalize flex items-center gap-1.5 mt-0.5">
            <span className={`w-2 h-2 rounded-full ${statusClass}`}></span>
            {project.status.replace(/_/g, ' ')}
          </span>
        </div>
        <div className="text-muted text-xs pr-4">
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
