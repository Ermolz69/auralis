import { createContext, useContext } from 'react';
import type { LibraryAsset, LibraryState, LibraryTool } from '@/shared/api/contracts/library';
import type { Project } from '@/shared/api/contracts/project';
export type LibraryContextValue = {
  state: LibraryState;
  projects: Project[];
  loading: boolean;
  error: string | null;
  view: 'library' | 'workspace' | 'settings';
  workspaceId: string | null;
  setView: (view: 'library' | 'workspace' | 'settings') => void;
  activateWorkspace: (id: string) => void;
  mutate: (change: (state: LibraryState) => void) => Promise<void>;
  refresh: () => Promise<void>;
  openProject: (id: string) => Promise<void>;
  openAsset: (asset: LibraryAsset, tool?: LibraryTool, projectId?: string | null) => Promise<void>;
  openTool: (tool: LibraryTool, assetId?: string | null) => Promise<void>;
};
export const LibraryContext = createContext<LibraryContextValue | null>(null);
export function useLibrary() {
  const value = useContext(LibraryContext);
  if (!value) throw new Error('LibraryProvider is required');
  return value;
}
