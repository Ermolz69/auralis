import { createContext } from 'react';
import type { Project } from './types';
import type { ProjectSelection } from './selection';

export interface OperationToken {
  readonly generation: number;
  readonly projectId: string | null;
}

export interface ProjectContextType {
  readonly selection: ProjectSelection;
  readonly projectId: string | null;
  readonly project: Project | null;
  setProject: (project: Project | null) => void;
  deletingProjectId: string | null;
  beginProjectDeletion: (id: string) => boolean;
  finishProjectDeletion: (id: string) => void;
  operationGeneration: number;
  captureToken: () => OperationToken;
  validateToken: (token: OperationToken) => boolean;
}

export const ProjectContext = createContext<ProjectContextType | undefined>(undefined);
