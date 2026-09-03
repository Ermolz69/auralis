import { useContext, useMemo } from 'react';
import { JobContext } from './context';
import { isActiveJobStatus } from './formatters';
import { selectProjectJobs } from './selectors';

function useJobStore() {
  const context = useContext(JobContext);
  if (!context) {
    throw new Error('useJobContext must be used within a JobProvider');
  }

  return context;
}

export function useJobContext() {
  const context = useJobStore();
  const activeJobs = useMemo(
    () => Object.values(context.jobs).filter((job) => isActiveJobStatus(job.status)),
    [context.jobs],
  );
  const completedJobs = useMemo(
    () => Object.values(context.jobs).filter((job) => !isActiveJobStatus(job.status)),
    [context.jobs],
  );

  return {
    ...context,
    activeJobs,
    completedJobs,
  };
}

export function useProjectJobs(projectId: string | null) {
  const { jobs: allJobs, phase, pendingRefetch } = useJobStore();
  const jobs = useMemo(() => selectProjectJobs(allJobs, projectId), [allJobs, projectId]);
  const activeJobs = useMemo(() => jobs.filter((job) => isActiveJobStatus(job.status)), [jobs]);
  const completedJobs = useMemo(() => jobs.filter((job) => !isActiveJobStatus(job.status)), [jobs]);

  return { jobs, activeJobs, completedJobs, phase, pendingRefetch };
}
