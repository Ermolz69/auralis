import type { RefObject } from 'react';
import type { Project } from '@/entities/project';
import { ProjectListRow } from './ProjectListRow';

type ProjectListRowsProps = {
  projects: Project[];
  deletingProjectId: string | null;
  openButtonRefs: RefObject<Map<string, HTMLButtonElement>>;
  deleteButtonRefs: RefObject<Map<string, HTMLButtonElement>>;
  onOpen: (project: Project) => void;
  onDelete: (project: Project) => void;
  onRename?: (project: Project, title: string) => void;
  onOpenFolder?: (project: Project) => void;
};

export function ProjectListRows({
  projects,
  deletingProjectId,
  openButtonRefs,
  deleteButtonRefs,
  onOpen,
  onDelete,
  onRename,
  onOpenFolder,
}: ProjectListRowsProps) {
  return (
    <div className="flex min-w-0 flex-col gap-0.5 overflow-y-auto">
      {projects.map((project) => (
        <ProjectListRow
          key={project.id}
          project={project}
          isDeleting={deletingProjectId === project.id}
          isAnyDeleting={deletingProjectId !== null}
          openButtonRef={(el) => {
            if (el) openButtonRefs.current.set(project.id, el);
            else openButtonRefs.current.delete(project.id);
          }}
          deleteButtonRef={(el) => {
            if (el) deleteButtonRefs.current.set(project.id, el);
            else deleteButtonRefs.current.delete(project.id);
          }}
          onOpen={onOpen}
          onDelete={onDelete}
          onRename={onRename}
          onOpenFolder={onOpenFolder}
        />
      ))}
    </div>
  );
}
