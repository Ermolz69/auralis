import { useEffect, useReducer, useRef } from 'react';
import type { ReactNode } from 'react';
import { JobContext } from './context';
import { jobStoreReducer, initializeStore } from './reducer';
import { JobStoreSynchronizer } from './synchronization';

export function JobProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(jobStoreReducer, undefined, initializeStore);

  const synchronizerRef = useRef<JobStoreSynchronizer | null>(null);
  if (!synchronizerRef.current) {
    synchronizerRef.current = new JobStoreSynchronizer(dispatch);
  }

  // Synchronization belongs to the app lifetime, independent of project selection.
  useEffect(() => {
    const sync = synchronizerRef.current;
    if (sync) {
      void sync.startCycle();
    }
    return () => {
      sync?.dispose();
    };
  }, []);

  // Handle follow-up fetches triggered by pendingRefetch from the reducer
  useEffect(() => {
    if (state.pendingRefetch) {
      synchronizerRef.current?.requestFetch(state.generation);
    }
  }, [state.pendingRefetch, state.generation]);

  return <JobContext.Provider value={state}>{children}</JobContext.Provider>;
}
