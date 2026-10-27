import { createContext, useContext } from 'react';
import type { LibraryTool } from '@/shared/api/contracts/library';
export type ActionTarget = {
  projectId: string | null;
  assetId: string | null;
  workspaceId?: string | null;
};
export type StudioAction =
  | 'new-project'
  | 'new-text'
  | 'import'
  | 'youtube'
  | 'settings'
  | 'exit'
  | 'open-project'
  | 'rename-project'
  | 'delete-project'
  | 'open-asset'
  | 'rename-asset'
  | 'properties'
  | 'attach'
  | 'remove-asset'
  | `tool:${LibraryTool}`;
export type StudioActions = {
  select: (target: ActionTarget) => void;
  run: (action: StudioAction, target?: ActionTarget) => void;
  context: (event: React.MouseEvent | React.KeyboardEvent, target: ActionTarget) => void;
  drop: (assetId: string, projectId: string) => void;
};
export const StudioActionsContext = createContext<StudioActions>({
  select: () => {},
  run: () => {},
  context: () => {},
  drop: () => {},
});
export const useStudioActions = () => useContext(StudioActionsContext);
