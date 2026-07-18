import { useContext, useMemo } from 'react';
import { JobContext } from './JobProvider';
import type { JobDto } from './types';

export function useJobContext() {
  const context = useContext(JobContext);
  if (!context) {
    throw new Error('useJobContext must be used within a JobProvider');
  }

  const activeJobs = useMemo(() => {
    return Object.values(context.jobs).filter(
      (j: JobDto) => j.status !== 'completed' && j.status !== 'failed' && j.status !== 'cancelled',
    );
  }, [context.jobs]);

  const completedJobs = useMemo(() => {
    return Object.values(context.jobs).filter(
      (j: JobDto) => j.status === 'completed' || j.status === 'failed' || j.status === 'cancelled',
    );
  }, [context.jobs]);

  return {
    ...context,
    activeJobs,
    completedJobs,
  };
}
