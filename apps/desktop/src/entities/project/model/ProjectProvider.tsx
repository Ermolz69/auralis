import { useState, useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
import { toCommandError } from '@/shared/api/contracts';
import { ProjectContext } from './context';
import type { Project } from './types';
import type { OperationToken } from './context';
import { subscribeProjectChanges } from './projectChanges';
import type { ProjectSelection } from './selection';

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [selection, setSelection] = useState<ProjectSelection>({ status: 'closed' });
  const project = selection.status === 'open' ? selection.project : null;
  const projectId = project?.id ?? null;
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);
  const [operationGeneration, setOperationGeneration] = useState<number>(0);

  const deletingProjectIdRef = useRef<string | null>(null);
  const operationGenerationRef = useRef<number>(0);
  const selectionRef = useRef<ProjectSelection>(selection);
  const listenerFetchSequence = useRef(0);
  const currentProjectId = () =>
    selectionRef.current.status === 'open' ? selectionRef.current.project.id : null;

  const invalidateOperations = () => {
    const nextGeneration = operationGenerationRef.current + 1;
    operationGenerationRef.current = nextGeneration;
    setOperationGeneration(nextGeneration);
  };

  const setProject = (nextProject: Project | null) => {
    if (nextProject === null || nextProject.id !== currentProjectId()) {
      invalidateOperations();
    }
    listenerFetchSequence.current += 1;
    const next: ProjectSelection = nextProject
      ? { status: 'open', project: nextProject }
      : { status: 'closed' };
    selectionRef.current = next;
    setSelection(next);
  };

  const beginProjectDeletion = (id: string) => {
    if (deletingProjectIdRef.current !== null) return false;
    invalidateOperations();
    deletingProjectIdRef.current = id;
    setDeletingProjectId(id);
    return true;
  };

  const finishProjectDeletion = (id: string) => {
    if (deletingProjectIdRef.current === id) {
      deletingProjectIdRef.current = null;
      setDeletingProjectId(null);
    }
  };

  const captureToken = (): OperationToken => {
    return {
      generation: operationGenerationRef.current,
      projectId: currentProjectId(),
    };
  };

  const validateToken = (token: OperationToken): boolean => {
    return (
      deletingProjectIdRef.current === null &&
      token.generation === operationGenerationRef.current &&
      token.projectId === currentProjectId()
    );
  };

  useEffect(() => {
    if (!projectId) return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const unsubscribe = subscribeProjectChanges((change) => {
      const id = change.type === 'updated' ? change.project.id : change.projectId;
      if (id !== projectId || currentProjectId() !== id) return;
      listenerFetchSequence.current += 1;
      if (change.type === 'removed') {
        setProject(null);
      } else if (deletingProjectIdRef.current === null) {
        setProject(change.project);
      }
    });

    const setupListener = async () => {
      try {
        const fn = await listen<{ projectId: string }>('project-updated', async (event) => {
          if (event.payload.projectId === projectId) {
            if (deletingProjectIdRef.current !== null) {
              return;
            }

            const token = captureToken();
            const currentFetchSeq = ++listenerFetchSequence.current;

            try {
              const updatedProject = await invoke('get_project_cmd', { projectId });

              if (
                cancelled ||
                currentFetchSeq !== listenerFetchSequence.current ||
                !validateToken(token) ||
                currentProjectId() !== projectId ||
                event.payload.projectId !== projectId
              ) {
                return;
              }

              if (updatedProject.id === projectId) setProject(updatedProject);
            } catch (e) {
              if (
                cancelled ||
                currentFetchSeq !== listenerFetchSequence.current ||
                !validateToken(token) ||
                currentProjectId() !== projectId ||
                event.payload.projectId !== projectId
              ) {
                return;
              }

              const cmdErr = toCommandError(e);
              if (cmdErr.code === 'NOT_FOUND') {
                setProject(null);
                console.warn('Project no longer exists:', cmdErr.message);
              } else {
                console.error('Failed to sync project:', cmdErr);
              }
            }
          }
        });

        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      } catch (err) {
        console.warn('Failed to listen to project-updated event:', toCommandError(err));
      }
    };

    setupListener();

    return () => {
      cancelled = true;
      unsubscribe();
      if (unlisten) unlisten();
    };
  }, [projectId]);

  return (
    <ProjectContext.Provider
      value={{
        selection,
        projectId,
        project,
        setProject,
        deletingProjectId,
        beginProjectDeletion,
        finishProjectDeletion,
        operationGeneration,
        captureToken,
        validateToken,
      }}
    >
      {children}
    </ProjectContext.Provider>
  );
}
