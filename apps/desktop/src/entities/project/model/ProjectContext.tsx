import { createContext, useContext, useState } from 'react';
import type { ReactNode } from 'react';

interface ProjectContextType {
  projectId: string | null;
  setProjectId: (id: string | null) => void;
  currentView: 'home' | 'project' | 'settings';
  setCurrentView: (view: 'home' | 'project' | 'settings') => void;
}

const ProjectContext = createContext<ProjectContextType | undefined>(undefined);

export function ProjectProvider({ children }: { children: ReactNode }) {
  const [projectId, setProjectId] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<'home' | 'project' | 'settings'>('home');

  return (
    <ProjectContext.Provider value={{ projectId, setProjectId, currentView, setCurrentView }}>
      {children}
    </ProjectContext.Provider>
  );
}

export function useProjectContext() {
  const context = useContext(ProjectContext);
  if (!context) {
    throw new Error('useProjectContext must be used within a ProjectProvider');
  }
  return context;
}
