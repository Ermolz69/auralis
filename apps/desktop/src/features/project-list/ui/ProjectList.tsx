import { useEffect, useState, useCallback, useRef, useLayoutEffect } from 'react';
import { deleteProject, listProjects, useProjectContext } from '@/entities/project';
import { listen } from '@/shared/api/tauri';
import type { Project } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { toast } from '@/shared/ui/toast';
import { toCommandError } from '@/shared/api/contracts';
import { DeleteProjectDialog } from './DeleteProjectDialog';
import { ProjectListRow } from './ProjectListRow';

export const ProjectList = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [projectToDelete, setProjectToDelete] = useState<Project | null>(null);
  const {
    projectId: currentProjectId,
    setProjectId,
    setProject,
    deletingProjectId,
    beginProjectDeletion,
    finishProjectDeletion,
  } = useProjectContext();
  const { setCurrentView } = useNavigation();

  const fetchGenerationRef = useRef(0);
  const pendingFocusTargetRef = useRef<{
    deletedIndex: number;
    deletedProjectId: string;
    reason: 'success' | 'cancel' | 'error';
  } | null>(null);

  // Sync refs during render to prevent stale windows
  const deletingProjectIdRef = useRef(deletingProjectId);
  deletingProjectIdRef.current = deletingProjectId;

  const currentProjectIdRef = useRef(currentProjectId);
  currentProjectIdRef.current = currentProjectId;

  // Refs for focusing elements
  const deleteButtonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const openButtonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const headingRef = useRef<HTMLHeadingElement>(null);

  const clearProjectContextIfCurrent = (deletedProjectId: string) => {
    if (currentProjectIdRef.current !== deletedProjectId) return;
    setProjectId(null);
    setProject(null);
    setCurrentView('home');
  };

  const fetchProjects = useCallback(async () => {
    fetchGenerationRef.current += 1;
    const currentGen = fetchGenerationRef.current;

    try {
      const data = await listProjects();
      if (currentGen === fetchGenerationRef.current) {
        setProjects(data);
        setIsLoading(false);
      }
    } catch (e) {
      if (currentGen === fetchGenerationRef.current) {
        console.error('Failed to fetch projects', e);
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    fetchProjects();

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
    setProjectId(project.id);
    setProject(project);
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

    // Invalidate already running list requests by incrementing fetchGen immediately after lock acquisition
    fetchGenerationRef.current += 1;

    setProjectToDelete(null); // Close dialog

    try {
      await deleteProject(project.id);

      pendingFocusTargetRef.current = {
        deletedIndex,
        deletedProjectId: project.id,
        reason: 'success',
      };
      setProjects((prev) => prev.filter((p) => p.id !== project.id));
      clearProjectContextIfCurrent(project.id);

      // Separate Refetch
      await fetchProjects();
    } catch (error) {
      const commandError = toCommandError(error);
      const errorMessage = commandError.message;
      const errorCode = commandError.code;

      if (errorCode === 'NOT_FOUND') {
        pendingFocusTargetRef.current = {
          deletedIndex,
          deletedProjectId: project.id,
          reason: 'success',
        };
        setProjects((prev) => prev.filter((p) => p.id !== project.id));
        clearProjectContextIfCurrent(project.id);

        await fetchProjects();
        toast.success('Project was already removed');
      } else if (errorCode === 'CONFLICT' || errorCode === 'BUSY') {
        pendingFocusTargetRef.current = {
          deletedIndex,
          deletedProjectId: project.id,
          reason: 'error',
        };
        toast.warning(errorMessage);
        await fetchProjects();
      } else {
        pendingFocusTargetRef.current = {
          deletedIndex,
          deletedProjectId: project.id,
          reason: 'error',
        };
        toast.error(errorMessage);
      }
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

  if (isLoading) {
    return (
      <div className="text-muted text-sm text-center py-4 animate-pulse">Loading projects...</div>
    );
  }

  return (
    <div className="w-full flex flex-col gap-3 mt-8">
      <DeleteProjectDialog
        project={projectToDelete}
        isDeleting={deletingProjectId !== null}
        onCancel={cancelDelete}
        onConfirm={() => void executeDelete()}
      />
      <h3
        ref={headingRef}
        tabIndex={-1}
        className="text-sm font-semibold text-muted uppercase tracking-wider mb-2 text-left focus:outline-none focus:text-text"
      >
        Recent Projects
      </h3>
      <div className="flex flex-col gap-2 max-h-[40vh] overflow-y-auto pr-2 custom-scrollbar">
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
            onOpen={handleOpenProject}
            onDelete={handleDeleteClick}
          />
        ))}
      </div>
    </div>
  );
};
