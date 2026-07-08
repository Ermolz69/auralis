import { useState, useEffect } from 'react';
import type { ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
import { isCommandError } from '@/shared/api/contracts';
import { ProjectContext } from './context';
import type { Project } from './types';

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [projectId, setProjectId] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);

  useEffect(() => {
    if (!projectId) return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        const fn = await listen<{ projectId: string }>('project-updated', async (event) => {
          if (event.payload.projectId === projectId) {
            try {
              const updatedProject = await invoke('get_project_cmd', { projectId });
              setProject(updatedProject);
            } catch (e) {
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
    <ProjectContext.Provider value={{ projectId, setProjectId, project, setProject }}>
      {children}
    </ProjectContext.Provider>
  );
}
