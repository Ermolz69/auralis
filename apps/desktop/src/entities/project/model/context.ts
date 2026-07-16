import { createContext } from 'react';
import type { Project } from './types';

export interface ProjectContextType {
  projectId: string | null;
  setProjectId: (id: string | null) => void;
  project: Project | null;
  setProject: (project: Project | null) => void;
  deletingProjectId: string | null;
  beginProjectDeletion: (id: string) => boolean;
  finishProjectDeletion: (id: string) => void;
}

export const ProjectContext = createContext<ProjectContextType | undefined>(undefined);
