import { expect, test } from 'vitest';
import type { Job, JobEvent, JobStatus, JobStage } from './job';

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

import contract from '../../../../../../tests/fixtures/job_contract.json';

// We hardcode the exhaustive list of statuses and stages here
// to ensure the TS union types match the cross-language contract exactly.
// If the contract adds a new stage, this test will fail until TS is updated.
const statuses: JobStatus[] = [
  'pending',
  'running',
  'completed',
  'failed',
  'cancelled',
];

const stages: JobStage[] = [
  'validateSource',
  'inspectSubtitles',
  'fetchMetadata',
  'downloadMedia',
  'extractOrGenerateTranscript',
  'segmentTranscript',
  'translateTranscript',
  'prepareDubbingScript',
  'synthesizeSegments',
  'postprocessAudio',
  'muxAudioTrack',
  'exportResult',
];

test('Job Contracts match the cross-language statuses and stages', () => {
  // Assert length to prevent missing variants
  expect(statuses.length).toBe(contract.statuses.length);
  for (const status of contract.statuses) {
    expect(statuses).toContain(status as JobStatus);
  }

  // Assert length to prevent missing variants
  expect(stages.length).toBe(contract.stages.length);
  for (const stage of contract.stages) {
    expect(stages).toContain(stage as JobStage);
  }
  
  // Verify example payload can be typed as JobEvent
  const example: JobEvent = contract.examplePayload as JobEvent;
  expect(example.status).toBe('running');
});
