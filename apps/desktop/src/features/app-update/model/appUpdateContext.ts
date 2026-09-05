import { createContext, useContext } from 'react';

export type AppUpdatePhase =
  | 'idle'
  | 'unsupported'
  | 'checking'
  | 'upToDate'
  | 'available'
  | 'downloading'
  | 'restarting'
  | 'error';

export interface AppUpdateInfo {
  version: string;
  date?: string;
  notes?: string;
}

export interface AppUpdateProgress {
  downloadedBytes: number;
  totalBytes?: number;
  percent?: number;
}

export interface AppUpdateState {
  phase: AppUpdatePhase;
  currentVersion: string;
  update: AppUpdateInfo | null;
  progress: AppUpdateProgress | null;
  error: string | null;
}

export interface AppUpdateContextValue extends AppUpdateState {
  checkForUpdates: () => Promise<void>;
  installUpdate: () => Promise<void>;
}

export const AppUpdateContext = createContext<AppUpdateContextValue | null>(null);

export function useAppUpdate() {
  const value = useContext(AppUpdateContext);
  if (!value) throw new Error('useAppUpdate must be used inside AppUpdateProvider');
  return value;
}
