import { useState, useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
import { isCommandError } from '@/shared/api/contracts';
import { ProjectContext } from './context';
import type { Project } from './types';

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [projectId, setProjectId] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [deletingProjectId, setDeletingProjectId] = useState<string | null>(null);

  const deletingProjectIdRef = useRef<string | null>(null);
  const fetchGenerationRef = useRef<number>(0);

  const beginProjectDeletion = (id: string) => {
    if (deletingProjectIdRef.current !== null) return false;
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

  useEffect(() => {
    if (!projectId) return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        const fn = await listen<{ projectId: string }>('project-updated', async (event) => {
          if (event.payload.projectId === projectId) {
            if (deletingProjectIdRef.current === projectId) {
              return; // Do not fetch if currently deleting
            }
            
            fetchGenerationRef.current += 1;
            const currentGen = fetchGenerationRef.current;

            try {
              const updatedProject = await invoke('get_project_cmd', { projectId });
              
              if (cancelled || currentGen !== fetchGenerationRef.current || projectId !== event.payload.projectId) {
                return;
              }
              
              if (deletingProjectIdRef.current === projectId) {
                return;
              }

              setProject(updatedProject);
            } catch (e) {
              if (cancelled || currentGen !== fetchGenerationRef.current) return;
              
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
    <ProjectContext.Provider value={{
      projectId, setProjectId,
      project, setProject,
      deletingProjectId, beginProjectDeletion, finishProjectDeletion
    }}>
      {children}
    </ProjectContext.Provider>
  );
}
