import type {
  Job as ContractJob,
  JobEvent as ContractJobEvent,
  JobEventKind,
  JobProgress,
} from '@/shared/api/contracts/job';
export type { JobKind, JobStatus } from '@/shared/api/contracts/job';

export type JobLifecycleEventKindDto = JobEventKind;
export type JobProgressDto = JobProgress;
export type JobDto = ContractJob;
export type Job = ContractJob;
export type JobEvent = ContractJobEvent;
export type JobEventDto = ContractJobEvent;

export type JobStoreState = {
  phase: 'idle' | 'initializing' | 'synchronizing' | 'ready' | 'stale';
  jobs: Record<string, JobDto>;
  buffer: JobEventDto[];
  pendingRefetch: boolean;
  generation: number;
};

export type JobSynchronizationConfig = {
  maxBufferedEvents: number;
  retryInitialMs: number;
  retryMaxMs: number;
  retryExponentLimit: number;
};

export const DEFAULT_JOB_SYNCHRONIZATION_CONFIG = {
  maxBufferedEvents: 256,
  retryInitialMs: 1000,
  retryMaxMs: 30000,
  retryExponentLimit: 5,
} satisfies JobSynchronizationConfig;
