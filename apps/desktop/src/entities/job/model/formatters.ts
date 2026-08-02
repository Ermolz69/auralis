import type { JobDto, JobStatus } from './types';

export type JobStatusTone = 'success' | 'danger' | 'default' | 'warning';

const JOB_STATUS_LABELS = {
  pending: 'Waiting to start',
  running: 'Running',
  completed: 'Completed',
  failed: 'Failed',
  cancelled: 'Cancelled',
} satisfies Record<JobStatus, string>;

const JOB_STAGE_LABELS: Record<string, string> = {
  validateSource: 'Validating source',
  inspectSubtitles: 'Inspecting subtitles',
  fetchMetadata: 'Reading media metadata',
  downloadMedia: 'Downloading media',
  extractOrGenerateTranscript: 'Preparing transcript',
  segmentTranscript: 'Segmenting transcript',
  translateTranscript: 'Translating transcript',
  prepareDubbingScript: 'Preparing dubbing script',
  synthesizeSegments: 'Synthesizing voice segments',
  postprocessAudio: 'Post-processing audio',
  muxAudioTrack: 'Muxing audio track',
  exportResult: 'Exporting result',
  importYoutubeSubtitles: 'Importing YouTube subtitles',
};

export const formatJobStage = (stage: string | null): string => {
  if (!stage) return '';

  const knownLabel = JOB_STAGE_LABELS[stage];
  if (knownLabel) return knownLabel;

  const words = stage
    .trim()
    .replace(/[_-]+/g, ' ')
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/\s+/g, ' ')
    .toLowerCase();

  return words ? words.charAt(0).toUpperCase() + words.slice(1) : 'Current step';
};

export const formatJobStatus = (job: Pick<JobDto, 'status' | 'stage'>): string => {
  const statusLabel = JOB_STATUS_LABELS[job.status];
  const stageLabel = formatJobStage(job.stage);

  if ((job.status === 'pending' || job.status === 'running') && stageLabel) {
    return `${statusLabel}: ${stageLabel}`;
  }

  return statusLabel;
};

export const getJobStatusTone = (status: JobStatus): JobStatusTone => {
  switch (status) {
    case 'completed':
      return 'success';
    case 'failed':
      return 'danger';
    case 'cancelled':
      return 'warning';
    case 'pending':
    case 'running':
      return 'default';
  }
};

export const isActiveJobStatus = (status: JobStatus): boolean =>
  status === 'pending' || status === 'running';
