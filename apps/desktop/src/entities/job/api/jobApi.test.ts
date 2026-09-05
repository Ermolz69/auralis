import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { Job, JobEvent } from '../model/types';
import {
  cancelJob,
  getJobsSnapshot,
  listJobs,
  subscribeJobEvents,
  subscribeJobsInvalidated,
} from './jobApi';
import { invoke, listen } from '@/shared/api/tauri';

vi.mock('@/shared/api/tauri', () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

describe('jobApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('listJobs and cancelJob preserve command results and arguments', async () => {
    const job = createJob();
    vi.mocked(invoke)
      .mockResolvedValueOnce([job] as never)
      .mockResolvedValueOnce(job as never);

    await expect(listJobs()).resolves.toEqual([job]);
    await expect(cancelJob('job-1')).resolves.toBe(job);

    expect(invoke).toHaveBeenNthCalledWith(1, 'list_jobs_cmd');
    expect(invoke).toHaveBeenNthCalledWith(2, 'cancel_job_cmd', { jobId: 'job-1' });
  });

  it('subscribeJobEvents forwards the typed event payload', async () => {
    const handler = vi.fn();
    const jobEvent: JobEvent = { kind: 'progressed', job: createJob() };
    vi.mocked(listen).mockResolvedValue(vi.fn());

    await subscribeJobEvents(handler);

    expect(listen).toHaveBeenCalledWith('job-event', expect.any(Function));
    const listener = vi.mocked(listen).mock.calls[0]?.[1];
    listener?.({ event: 'job-event', id: 1, payload: jobEvent });
    expect(handler).toHaveBeenCalledExactlyOnceWith(jobEvent);
  });

  it('subscribeJobsInvalidated invokes the handler without exposing transport details', async () => {
    const handler = vi.fn();
    vi.mocked(listen).mockResolvedValue(vi.fn());

    await subscribeJobsInvalidated(handler);

    expect(listen).toHaveBeenCalledWith('job-events-invalidated', expect.any(Function));
    const listener = vi.mocked(listen).mock.calls[0]?.[1];
    listener?.({ event: 'job-events-invalidated', id: 2, payload: null });
    expect(handler).toHaveBeenCalledOnce();
  });

  it('getJobsSnapshot invokes list_jobs_snapshot_cmd with projectId', async () => {
    vi.mocked(invoke).mockResolvedValue([] as never);

    await getJobsSnapshot('project-1');

    expect(invoke).toHaveBeenCalledWith('list_jobs_snapshot_cmd', {
      projectId: 'project-1',
    });
  });
});

function createJob(): Job {
  return {
    id: 'job-1',
    kind: 'dubbing',
    revision: 1,
    projectId: 'project-1',
    title: 'Dubbing',
    status: 'running',
    stage: 'translateTranscript',
    progress: {
      percent: 50,
      message: 'Translating',
      currentStep: 'translateTranscript',
      processedItems: 5,
      totalItems: 10,
    },
    error: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:01:00Z',
  };
}
