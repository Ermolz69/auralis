export type JobStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type JobStage =
  | 'validate_source'
  | 'fetch_metadata'
  | 'prepare_media'
  | 'generate_transcript'
  | 'finalize';

export type Job = {
  id: string;
  title: string;
  status: JobStatus;
  stage: JobStage | null;
  progress: {
    percent: number;
  };
  error: string | null;
  createdAt: string;
  updatedAt: string;
};

export type JobEvent = {
  jobId: string;
  status: JobStatus;
  stage: JobStage | null;
  progress: {
    percent: number;
  };
  message: string | null;
  error: string | null;
};
