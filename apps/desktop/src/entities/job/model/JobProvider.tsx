import { createContext, useEffect, useReducer, useRef } from 'react';
import type { ReactNode } from 'react';
import { jobStoreReducer, initializeStore } from './reducer';
import { JobStoreSynchronizer } from './synchronization';
import type { JobStoreState } from './types';

export const JobContext = createContext<JobStoreState | null>(null);

export function JobProvider({
  projectId,
  children,
}: {
  projectId: string | null;
  children: ReactNode;
}) {
  const [state, dispatch] = useReducer(jobStoreReducer, projectId, initializeStore);

  const stateRef = useRef<JobStoreState>(state);
  stateRef.current = state;

  const synchronizerRef = useRef<JobStoreSynchronizer | null>(null);
  if (!synchronizerRef.current) {
    synchronizerRef.current = new JobStoreSynchronizer(dispatch, () => stateRef.current);
  }

  // Handle mount, project switch, and unmount
  useEffect(() => {
    const sync = synchronizerRef.current;
    if (sync) {
      void sync.startCycle(projectId);
    }
    return () => {
      sync?.dispose();
    };
  }, [projectId]);

  // Handle follow-up fetches triggered by pendingRefetch from the reducer
  useEffect(() => {
    if (state.pendingRefetch) {
      synchronizerRef.current?.requestFetch(state.generation);
    }
  }, [state.pendingRefetch, state.generation]);

  return <JobContext.Provider value={state}>{children}</JobContext.Provider>;
}
