import { expect, test } from 'vitest';
import type { Job, JobEvent } from './job';

test('Job and JobEvent types match backend DTO shapes', () => {
  const mockJob = {
    id: 'job-1',
    projectId: 'proj-1',
    title: 'Test Job',
    status: 'running',
    stage: 'extractOrGenerateTranscript',
    progress: {
      percent: 45,
      message: 'Extracting audio...',
      currentStep: 'audio_extraction',
      processedItems: 1,
      totalItems: 2,
    },
    error: null,
    createdAt: '2023-01-01T00:00:00Z',
    updatedAt: '2023-01-01T00:00:00Z',
  } satisfies Job;

  const mockJobEvent = {
    jobId: 'job-1',
    projectId: 'proj-1',
    status: 'failed',
    stage: 'muxAudioTrack',
    progress: {
      percent: 90,
      message: 'Muxing failed',
      currentStep: 'mux',
      processedItems: null,
      totalItems: null,
    },
    error: 'FFmpeg error code 1',
  } satisfies JobEvent;

  expect(mockJob.status).toBe('running');
  expect(mockJobEvent.status).toBe('failed');
});
