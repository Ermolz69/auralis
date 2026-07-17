import { describe, it, expect, vi, beforeEach } from 'vitest';
import { JobStoreSynchronizer } from '../synchronization';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('JobStoreSynchronizer', () => {
  let dispatch: ReturnType<typeof vi.fn>;
  let getState: ReturnType<typeof vi.fn>;
  let synchronizer: JobStoreSynchronizer;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();

    let currentState = {
      phase: 'idle',
      generation: 1,
      scopeProjectId: 'proj1',
      jobs: {},
      buffer: [],
      pendingRefetch: false,
    };

    dispatch = vi.fn().mockImplementation((action: any) => {
       if (action.type === 'LISTENERS_REGISTERED') {
           currentState.phase = 'synchronizing';
       }
       if (action.type === 'FETCH_FAILED') {
           currentState.phase = 'stale';
       }
    });

    getState = vi.fn().mockImplementation(() => currentState);

    synchronizer = new JobStoreSynchronizer(dispatch as any, getState as any);
  });

  it('initializes and triggers fetch loop', async () => {
    (listen as any).mockResolvedValue(vi.fn());
    (invoke as any).mockResolvedValue([]);

    await synchronizer.startCycle('proj1');

    expect(dispatch).toHaveBeenCalledWith({ type: 'INITIALIZATION_CYCLE' });
    expect(listen).toHaveBeenCalledTimes(2);
    expect(dispatch).toHaveBeenCalledWith({ type: 'LISTENERS_REGISTERED' });
    expect(invoke).toHaveBeenCalledWith('list_jobs_snapshot_cmd', { projectId: 'proj1' });
    expect(dispatch).toHaveBeenCalledWith({
      type: 'SNAPSHOT_RESOLVED',
      generation: 1,
      projectId: 'proj1',
      jobs: [],
    });
  });
});
