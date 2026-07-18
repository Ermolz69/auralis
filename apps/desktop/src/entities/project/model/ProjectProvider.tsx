import { useState, useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
import { isCommandError } from '@/shared/api/contracts';
import { ProjectContext } from './context';
import type { Project } from './types';
import type { OperationToken } from './context';

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [projectId, setProjectIdState] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);
  const [operationGeneration, setOperationGeneration] = useState<number>(0);

  const deletingProjectIdRef = useRef<string | null>(null);
  const operationGenerationRef = useRef<number>(0);
  const currentProjectIdRef = useRef<string | null>(null);

  const invalidateOperations = () => {
    const nextGeneration = operationGenerationRef.current + 1;
    operationGenerationRef.current = nextGeneration;
    setOperationGeneration(nextGeneration);
  };

  const setProjectId = (id: string | null) => {
    if (id !== currentProjectIdRef.current) {
      currentProjectIdRef.current = id;
      invalidateOperations();
      setProjectIdState(id);
    }
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
    return { generation: operationGenerationRef.current };
  };

  const validateToken = (token: OperationToken): boolean => {
    return deletingProjectIdRef.current === null && token.generation === operationGenerationRef.current;
  };

  useEffect(() => {
    if (!projectId) return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let listenerFetchSequence = 0;

    const setupListener = async () => {
      try {
        const fn = await listen<{ projectId: string }>('project-updated', async (event) => {
          if (event.payload.projectId === projectId) {
            if (deletingProjectIdRef.current !== null) {
              return;
            }

            const token = captureToken();
            const currentFetchSeq = ++listenerFetchSequence;

            try {
              const updatedProject = await invoke('get_project_cmd', { projectId });

              if (
                cancelled ||
                currentFetchSeq !== listenerFetchSequence ||
                !validateToken(token) ||
                currentProjectIdRef.current !== projectId ||
                event.payload.projectId !== projectId
              ) {
                return;
              }

              setProject(updatedProject);
            } catch (e) {
              if (
                cancelled ||
                currentFetchSeq !== listenerFetchSequence ||
                !validateToken(token) ||
                currentProjectIdRef.current !== projectId ||
                event.payload.projectId !== projectId
              ) {
                return;
              }

              if (isCommandError(e) && e.code === 'NOT_FOUND') {
                setProject(null);
                console.warn('Project no longer exists:', e.message);
              } else {
                console.error('Failed to sync project:', e);
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
        console.warn('Failed to listen to project-updated event:', err);
      }
    };

    setupListener();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, [projectId]);

  return (
    <ProjectContext.Provider
      value={{
        projectId,
        setProjectId,
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

