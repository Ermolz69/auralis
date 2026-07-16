export type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export type JobStage =
  | 'validateSource'
  | 'inspectSubtitles'
  | 'fetchMetadata'
  | 'downloadMedia'
  | 'extractOrGenerateTranscript'
  | 'segmentTranscript'
  | 'translateTranscript'
  | 'prepareDubbingScript'
  | 'synthesizeSegments'
  | 'postprocessAudio'
  | 'muxAudioTrack'
  | 'exportResult';

export type JobProgress = {
  percent: number;
  message: string;
  currentStep: string | null;
  processedItems: number | null;
  totalItems: number | null;
};

export type Job = {
  id: string;
  revision: number;
  projectId: string | null;
  title: string;
  status: JobStatus;
  stage: JobStage | null;
  progress: JobProgress;
  error: string | null;
  createdAt: string;
  updatedAt: string;
};

export type JobEventKind = 'created' | 'started' | 'progressed' | 'completed' | 'failed' | 'cancelled';

export type JobEvent = {
  kind: JobEventKind;
  jobId: string;
  revision: number;
  projectId: string | null;
  status: JobStatus;
  stage: JobStage | null;
  progress: JobProgress;
  error: string | null;
};
