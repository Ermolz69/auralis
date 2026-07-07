import { useState, useEffect } from 'react';
import type { ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
import { ProjectContext } from './context';
import type { Project } from './types';

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [projectId, setProjectId] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);
  const [currentView, setCurrentView] = useState<'home' | 'project' | 'settings'>('home');

  useEffect(() => {
    if (!projectId) return;

    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      const fn = await listen<{ projectId: string }>('project-updated', async (event) => {
        if (event.payload.projectId === projectId) {
          try {
            const updatedProject = await invoke('get_project_cmd', { projectId });
            if (updatedProject) {
              setProject(updatedProject);
            }
          } catch (e) {
            console.error('Failed to sync project:', e);
          }
        }
      });
      unlisten = fn;
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [projectId]);

  return (
    <ProjectContext.Provider
      value={{ projectId, setProjectId, project, setProject, currentView, setCurrentView }}
    >
      {children}
    </ProjectContext.Provider>
  );
}
