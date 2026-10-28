import { useState } from 'react';
import {
  deleteProject,
  openProjectFolder,
  removeProjectPreferences,
  renameProject,
  projectUpdated,
  projectRemoved,
  useProjectContext,
} from '@/entities/project';
import type { Project } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { toast } from '@/shared/ui/toast';
import { toCommandError } from '@/shared/api/contracts';
import type { useProjectListData } from './useProjectListData';
import type { useProjectListFocus } from './useProjectListFocus';

type ActionDependencies = {
  context: ReturnType<typeof useProjectContext>;
  data: ReturnType<typeof useProjectListData>;
  focus: ReturnType<typeof useProjectListFocus>;
  deletingProjectIdRef: { current: string | null };
  currentProjectIdRef: { current: string | null };
};

export function useProjectListActions({
  context,
  data,
  focus,
  deletingProjectIdRef,
  currentProjectIdRef,
}: ActionDependencies) {
  const [projectToDelete, setProjectToDelete] = useState<Project | null>(null);
  const { setProject, beginProjectDeletion, finishProjectDeletion } = context;
  const { projects, setProjects, fetchGenerationRef, fetchProjects } = data;
  const { pendingFocusTargetRef } = focus;
  const { setCurrentView, setPipelineStep = () => undefined } = useNavigation();

  const clearProjectContextIfCurrent = (deletedProjectId: string) => {
    if (currentProjectIdRef.current !== deletedProjectId) return;
    setProject(null);
    setCurrentView('home');
  };

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
      updated = await renameProject(project.id, title, project.revision);
    } catch (error) {
      const commandError = toCommandError(error);
      if (commandError.code === 'CONFLICT') {
        await fetchProjects();
        toast.error('Project changed. Review the current title and retry your rename.');
      } else {
        toast.error(commandError.message);
      }
      return;
    }
    fetchGenerationRef.current += 1;
    setProjects((items) =>
      items.map((item) =>
        item.id === updated.id && item.revision <= updated.revision ? updated : item,
      ),
    );
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

  return {
    projectToDelete,
    handleOpenProject,
    handleDeleteClick,
    executeDelete,
    cancelDelete,
    handleRename,
    handleOpenFolder,
  };
}
