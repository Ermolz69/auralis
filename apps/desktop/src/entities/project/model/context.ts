import { createContext } from 'react';
import type { Project } from './types';

export interface ProjectContextType {
  projectId: string | null;
  setProjectId: (id: string | null) => void;
  project: Project | null;
  setProject: (project: Project | null) => void;
}

export const ProjectContext = createContext<ProjectContextType | undefined>(undefined);
