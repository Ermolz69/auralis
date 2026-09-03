import { describe, expect, it } from 'vitest';
import type { JobDto, JobStatus } from '@/entities/job';
import { getPipelineStatus } from './pipelineStatus';

const job = (overrides: Partial<JobDto> = {}): JobDto => ({
  id: 'job-1',
  kind: 'dubbing',
  projectId: 'p1',
  revision: 1,
  title: 'Subtitle import',
  status: 'running',
  stage: null,
  progress: { percent: 0, message: '', currentStep: null, processedItems: null, totalItems: null },
  error: null,
  createdAt: '2026-09-03T10:00:00Z',
  updatedAt: '2026-09-03T10:00:00Z',
  ...overrides,
});

describe('pipeline status by job kind', () => {
  it.each(['pending', 'running', 'completed', 'failed', 'cancelled'] satisfies JobStatus[])(
    'ignores unrelated %s jobs even when their title and stage resemble subtitles',
    (status) => {
      const unrelated = job({ kind: 'export', status, stage: 'extractOrGenerateTranscript' });
      expect(getPipelineStatus(false, [unrelated])).toEqual({ source: 'idle', subtitles: 'idle' });
      expect(getPipelineStatus(true, [unrelated])).toEqual({
        source: 'completed',
        subtitles: 'idle',
      });
    },
  );

  it.each(['pending', 'running', 'completed', 'failed', 'cancelled'] satisfies JobStatus[])(
    'uses the %s status of the mapped kind only',
    (status) => {
      expect(
        getPipelineStatus(true, [
          job({ id: 'other-running', kind: 'export', status: 'running' }),
          job({ id: 'other-failed', kind: 'transcription', status: 'failed' }),
          job({ status }),
        ]),
      ).toEqual({ source: 'completed', subtitles: status });
    },
  );

  it('shows a successful retry instead of a previous error regardless of array order', () => {
    const failed = job({ id: 'failed', status: 'failed', updatedAt: '2026-09-03T12:00:00Z' });
    const retry = job({ id: 'retry', status: 'completed', createdAt: '2026-09-03T11:00:00Z' });
    for (const jobs of [
      [failed, retry],
      [retry, failed],
    ]) {
      expect(getPipelineStatus(true, jobs).subtitles).toBe('completed');
    }
  });

  it('prefers an active mapped job over terminal history', () => {
    const active = job({ status: 'pending' });
    const history = job({ id: 'history', status: 'completed', createdAt: '2026-09-03T11:00:00Z' });
    for (const jobs of [
      [active, history],
      [history, active],
    ]) {
      expect(getPipelineStatus(true, jobs).subtitles).toBe('pending');
    }
  });

  it('orders attempts by actual creation time, including timezone offsets', () => {
    const older = job({ status: 'failed', createdAt: '2026-09-03T13:00:00+03:00' });
    const latest = job({ id: 'latest', status: 'cancelled', createdAt: '2026-09-03T11:00:00Z' });
    expect(getPipelineStatus(true, [older, latest]).subtitles).toBe('cancelled');
  });

  it('uses job IDs to break timestamp ties consistently without mutating the snapshot', () => {
    const jobs = Object.freeze([
      job({ id: 'b', status: 'completed' }),
      job({ id: 'a', status: 'failed' }),
    ]);
    expect(getPipelineStatus(true, jobs).subtitles).toBe('completed');
    expect(getPipelineStatus(true, [...jobs].reverse()).subtitles).toBe('completed');
    expect(jobs.map((job) => job.id)).toEqual(['b', 'a']);
  });
});
