export type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
export type JobLifecycleEventKindDto = 'created' | 'started' | 'progressed' | 'completed' | 'failed' | 'cancelled';

export type JobProgressDto = {
  percent: number;
  message: string;
  currentStep: string | null;
  processedItems: number | null;
  totalItems: number | null;
};

export type JobDto = {
  id: string;
  revision: number;
  projectId: string | null;
  title: string;
  status: JobStatus;
  stage: string | null;
  progress: JobProgressDto;
  error: string | null;
  createdAt: string;
  updatedAt: string;
};

// Aliases for compatibility with the rest of the codebase
export type Job = JobDto;
export type JobEvent = JobEventDto;

export type JobEventDto = {
  kind: JobLifecycleEventKindDto;
  job: JobDto;
};

export type JobStoreState = {
  phase: 'idle' | 'initializing' | 'synchronizing' | 'ready' | 'stale';
  scopeProjectId: string | null;
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
