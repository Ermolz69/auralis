import { describe, it, expect, vi, beforeEach } from 'vitest';
import { subscribeJobEvents, subscribeJobsInvalidated, getJobsSnapshot } from './jobApi';
import { invoke, listen } from '@/shared/api/tauri';

vi.mock('@/shared/api/tauri', () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

describe('jobApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('subscribeJobEvents registers listener with canonical job-event string', async () => {
    const handler = vi.fn();
    (listen as any).mockResolvedValue(vi.fn());

    await subscribeJobEvents(handler);

    expect(listen).toHaveBeenCalledWith('job-event', expect.any(Function));
  });

  it('subscribeJobsInvalidated registers listener with canonical job-events-invalidated string', async () => {
    const handler = vi.fn();
    (listen as any).mockResolvedValue(vi.fn());

    await subscribeJobsInvalidated(handler);

    expect(listen).toHaveBeenCalledWith('job-events-invalidated', expect.any(Function));
  });

  it('getJobsSnapshot invokes list_jobs_snapshot_cmd with projectId', async () => {
    (invoke as any).mockResolvedValue([]);

    await getJobsSnapshot('project-1');

    expect(invoke).toHaveBeenCalledWith('list_jobs_snapshot_cmd', {
      projectId: 'project-1',
    });
  });
});
