import { useEffect, useState, useCallback, useRef, useLayoutEffect } from 'react';
import {
  deleteProject,
  listProjects,
  openProjectFolder,
  removeProjectPreferences,
  renameProject,
  projectUpdated,
  projectRemoved,
  useProjectContext,
} from '@/entities/project';
import { listen } from '@/shared/api/tauri';
import type { Project } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { toast } from '@/shared/ui/toast';
import { toCommandError } from '@/shared/api/contracts';
import { DeleteProjectDialog } from './DeleteProjectDialog';
import { ProjectListRows } from './ProjectListRows';
import { PendingYoutubeImports } from './PendingYoutubeImports';
import {
  ProjectListEmptyState,
  ProjectListErrorState,
  ProjectListLoadingState,
} from './ProjectListStates';

export const ProjectList = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [projectToDelete, setProjectToDelete] = useState<Project | null>(null);
  const {
    projectId: currentProjectId,
    setProject,
    deletingProjectId,
    beginProjectDeletion,
    finishProjectDeletion,
  } = useProjectContext();
  const { setCurrentView, setPipelineStep = () => undefined } = useNavigation();

  const fetchGenerationRef = useRef(0);
  const pendingFocusTargetRef = useRef<{
    deletedIndex: number;
    deletedProjectId: string;
    reason: 'success' | 'cancel' | 'error';
  } | null>(null);

  const deletingProjectIdRef = useRef(deletingProjectId);
  deletingProjectIdRef.current = deletingProjectId;

  const currentProjectIdRef = useRef(currentProjectId);
  currentProjectIdRef.current = currentProjectId;

  const deleteButtonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const openButtonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const headingRef = useRef<HTMLHeadingElement>(null);

  const clearProjectContextIfCurrent = (deletedProjectId: string) => {
    if (currentProjectIdRef.current !== deletedProjectId) return;
    setProject(null);
    setCurrentView('home');
  };

  const fetchProjects = useCallback(async (showLoading = false) => {
    fetchGenerationRef.current += 1;
    const currentGen = fetchGenerationRef.current;
    if (showLoading) setIsLoading(true);

    try {
      const data = await listProjects();
      if (currentGen === fetchGenerationRef.current) {
        setProjects(data);
        setListError(null);
        setIsLoading(false);
      }
    } catch (e) {
      if (currentGen === fetchGenerationRef.current) {
        const commandError = toCommandError(e);
        setListError(commandError.message);
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    fetchProjects(true);

    let unlistenProject: (() => void) | undefined;
    const setupListeners = async () => {
      try {
        unlistenProject = await listen<{ projectId: string }>('project-updated', (event) => {
          if (event.payload.projectId === deletingProjectIdRef.current) {
            return;
          }
          void fetchProjects();
        });
      } catch (e) {
        console.warn('Failed to setup Tauri listeners:', e);
      }
    };

    setupListeners();
    return () => {
      if (unlistenProject) unlistenProject();
    };
  }, [fetchProjects]);

  useLayoutEffect(() => {
    const target = pendingFocusTargetRef.current;
    if (!target) return;

    if (target.reason === 'cancel' || target.reason === 'error') {
      const btn = deleteButtonRefs.current.get(target.deletedProjectId);
      btn?.focus();
    } else if (target.reason === 'success') {
      if (projects.length === 0) {
        headingRef.current?.focus();
      } else {
        const nextIndex = Math.min(target.deletedIndex, projects.length - 1);
        const nextProjectId = projects[nextIndex]?.id;
        if (nextProjectId) {
          const btn = openButtonRefs.current.get(nextProjectId);
          btn?.focus();
        }
      }
    }
    pendingFocusTargetRef.current = null;
  });

  const handleOpenProject = (project: Project) => {
    if (deletingProjectIdRef.current !== null) return;
    setProject(project);
    setPipelineStep('source');
    setCurrentView('project');
  };

  const handleDeleteClick = (project: Project) => {
    if (deletingProjectIdRef.current !== null) return;
    setProjectToDelete(project);
  };

  const executeDelete = async () => {
    if (!projectToDelete) return;
    const project = projectToDelete;
    const deletedIndex = projects.findIndex((p) => p.id === project.id);

    if (!beginProjectDeletion(project.id)) {
      setProjectToDelete(null);
      return;
    }

    fetchGenerationRef.current += 1;

    setProjectToDelete(null);

    try {
      let alreadyRemoved = false;
      try {
        await deleteProject(project.id);
      } catch (error) {
        const commandError = toCommandError(error);
        if (commandError.code === 'NOT_FOUND') {
          alreadyRemoved = true;
        } else {
          pendingFocusTargetRef.current = {
            deletedIndex,
            deletedProjectId: project.id,
            reason: 'error',
          };
          if (commandError.code === 'CONFLICT' || commandError.code === 'BUSY') {
            toast.warning(commandError.message);
            await fetchProjects();
          } else {
            toast.error(commandError.message);
          }
          return;
        }
      }

      fetchGenerationRef.current += 1;
      pendingFocusTargetRef.current = {
        deletedIndex,
        deletedProjectId: project.id,
        reason: 'success',
      };
      setProjects((prev) => prev.filter((item) => item.id !== project.id));
      clearProjectContextIfCurrent(project.id);
      projectRemoved(project.id);
      const cleanup = removeProjectPreferences(project.id);
      if (!cleanup.persisted) {
        toast.warning('Project removed, but local preferences could not be cleaned up.');
      }
      await fetchProjects();
      if (alreadyRemoved) toast.success('Project was already removed');
    } finally {
      finishProjectDeletion(project.id);
    }
  };

  const cancelDelete = () => {
    if (projectToDelete) {
      const idx = projects.findIndex((p) => p.id === projectToDelete.id);
      pendingFocusTargetRef.current = {
        deletedIndex: idx,
        deletedProjectId: projectToDelete.id,
        reason: 'cancel',
      };
    }
    setProjectToDelete(null);
  };

  const handleRename = async (project: Project, title: string) => {
    let updated: Project;
    try {
      updated = await renameProject(project.id, title);
    } catch (error) {
      toast.error(toCommandError(error).message);
      return;
    }
    fetchGenerationRef.current += 1;
    setProjects((items) => items.map((item) => (item.id === updated.id ? updated : item)));
    if (currentProjectIdRef.current === updated.id) setProject(updated);
    projectUpdated(updated);
    toast.success('Project renamed');
  };

  const handleOpenFolder = async (project: Project) => {
    try {
      await openProjectFolder(project.id);
    } catch (error) {
      toast.error(toCommandError(error).message);
    }
  };

  return (
    <section className="flex w-full flex-col gap-3" aria-labelledby="recent-projects-heading">
      <PendingYoutubeImports onCompleted={() => void fetchProjects()} />
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
