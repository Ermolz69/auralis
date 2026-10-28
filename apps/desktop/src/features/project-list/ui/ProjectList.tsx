import { useRef } from 'react';
import { useProjectContext } from '@/entities/project';
import { DeleteProjectDialog } from './DeleteProjectDialog';
import { ProjectListRows } from './ProjectListRows';
import { PendingYoutubeImports } from './PendingYoutubeImports';
import { ArtifactRecovery } from './ArtifactRecovery';
import { useProjectListData } from '../model/useProjectListData';
import { useProjectListFocus } from '../model/useProjectListFocus';
import { useProjectListActions } from '../model/useProjectListActions';
import {
  ProjectListEmptyState,
  ProjectListErrorState,
  ProjectListLoadingState,
} from './ProjectListStates';

export const ProjectList = () => {
  const context = useProjectContext();
  const { projectId: currentProjectId, deletingProjectId } = context;

  const deletingProjectIdRef = useRef(deletingProjectId);
  deletingProjectIdRef.current = deletingProjectId;

  const currentProjectIdRef = useRef(currentProjectId);
  currentProjectIdRef.current = currentProjectId;

  const data = useProjectListData(deletingProjectIdRef);
  const { projects, isLoading, listError, fetchProjects } = data;
  const focus = useProjectListFocus(projects);
  const { deleteButtonRefs, openButtonRefs, headingRef } = focus;
  const {
    projectToDelete,
    handleOpenProject,
    handleDeleteClick,
    executeDelete,
    cancelDelete,
    handleRename,
    handleOpenFolder,
  } = useProjectListActions({ context, data, focus, deletingProjectIdRef, currentProjectIdRef });

  return (
    <section className="flex w-full flex-col gap-3" aria-labelledby="recent-projects-heading">
      <PendingYoutubeImports onCompleted={() => void fetchProjects()} />
      <ArtifactRecovery projects={projects} />
      <DeleteProjectDialog
        project={projectToDelete}
        isDeleting={deletingProjectId !== null}
        onCancel={cancelDelete}
        onConfirm={() => void executeDelete()}
      />
      <h3
        id="recent-projects-heading"
        ref={headingRef}
        tabIndex={-1}
        className="mb-1 text-left text-[11px] font-semibold uppercase tracking-wider text-muted focus:outline-none focus:text-text"
      >
        Recent projects
      </h3>
      {isLoading ? (
        <ProjectListLoadingState />
      ) : listError ? (
        <ProjectListErrorState error={listError} onRetry={() => void fetchProjects(true)} />
      ) : projects.length === 0 ? (
        <ProjectListEmptyState />
      ) : (
        <ProjectListRows
          projects={projects}
          deletingProjectId={deletingProjectId}
          openButtonRefs={openButtonRefs}
          deleteButtonRefs={deleteButtonRefs}
          onOpen={handleOpenProject}
          onDelete={handleDeleteClick}
          onRename={(project, title) => void handleRename(project, title)}
          onOpenFolder={(project) => void handleOpenFolder(project)}
        />
      )}
    </section>
  );
};
