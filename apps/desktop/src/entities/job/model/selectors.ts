import type { JobStoreState } from './types';

/** A closed project has no jobs; unattached jobs belong only to the global queue. */
export function selectProjectJobs(jobs: JobStoreState['jobs'], projectId: string | null) {
  return projectId === null ? [] : Object.values(jobs).filter((job) => job.projectId === projectId);
}
