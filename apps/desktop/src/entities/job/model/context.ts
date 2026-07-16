import { createContext } from 'react';
import type { Job } from './types';

export type JobContextValue = {
  jobs: Record<string, Job>;
  activeJobs: Job[];
  completedJobs: Job[];
};

export const JobContext = createContext<JobContextValue | null>(null);
