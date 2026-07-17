import { createContext, useEffect, useReducer, useRef } from 'react';
import type { ReactNode } from 'react';
import { jobStoreReducer, initializeStore } from './reducer';
import { JobStoreSynchronizer } from './synchronization';
import type { JobStoreState } from './types';

export const JobContext = createContext<JobStoreState | null>(null);

export function JobProvider({ 
  projectId, 
  children 
}: { 
  projectId: string | null; 
  children: ReactNode 
}) {
  const [state, dispatch] = useReducer(jobStoreReducer, projectId, initializeStore);
  
  const synchronizerRef = useRef<JobStoreSynchronizer | null>(null);

  if (!synchronizerRef.current) {
    synchronizerRef.current = new JobStoreSynchronizer(dispatch, () => state);
  }

  // Allow synchronizer to access latest state without creating stale closures
  useEffect(() => {
    synchronizerRef.current = new JobStoreSynchronizer(dispatch, () => state);
  }, [state, dispatch]);

  useEffect(() => {
    if (projectId !== state.scopeProjectId) {
      dispatch({ type: 'SWITCH_PROJECT', projectId });
    }
  }, [projectId, state.scopeProjectId]);

  useEffect(() => {
    const sync = synchronizerRef.current;
    if (sync && projectId) {
      sync.startCycle(projectId);
    }
    return () => {
      sync?.dispose();
    };
  }, [projectId]);

  return (
    <JobContext.Provider value={state}>
      {children}
    </JobContext.Provider>
  );
}
