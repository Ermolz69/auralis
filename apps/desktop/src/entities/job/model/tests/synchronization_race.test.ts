import { describe, it, expect, vi, beforeEach } from 'vitest';
import { JobStoreSynchronizer } from '../synchronization';
import { getJobsSnapshot, subscribeJobEvents, subscribeJobsInvalidated } from '../../api/jobApi';
import type { JobDto, JobEventDto, JobStoreState } from '../types';

vi.mock('../../api/jobApi', () => ({
  subscribeJobEvents: vi.fn(),
  subscribeJobsInvalidated: vi.fn(),
  getJobsSnapshot: vi.fn(),
}));

describe('JobStoreSynchronizer - Race Conditions', () => {
  let dispatch: ReturnType<typeof vi.fn>;
  let getState: ReturnType<typeof vi.fn>;
  let synchronizer: JobStoreSynchronizer;
  let currentState: JobStoreState;

  const createJob = (id: string, revision: number, projectId: string | null = 'p1'): JobDto => ({
    id,
    revision,
    projectId,
    title: `Job ${id}`,
    status: 'pending',
    stage: null,
    progress: {
      percent: 0,
      message: '',
      currentStep: null,
      processedItems: null,
      totalItems: null,
    },
    error: null,
    createdAt: '',
    updatedAt: '',
  });

  const createEvent = (job: JobDto, kind: any = 'created'): JobEventDto => ({
    kind,
    job,
  });

  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();

    currentState = {
      phase: 'idle',
      generation: 0,
      scopeProjectId: 'p1',
      jobs: {},
      buffer: [],
      pendingRefetch: false,
    };

    dispatch = vi.fn().mockImplementation((action: any) => {
      if (action.type === 'SWITCH_PROJECT') {
        currentState.generation = action.generation;
        currentState.scopeProjectId = action.projectId;
        currentState.phase = 'idle';
      }
      if (action.type === 'INITIALIZATION_CYCLE') {
        currentState.generation = action.generation;
        currentState.phase = 'initializing';
      }
      if (action.type === 'LISTENERS_REGISTERED') {
        currentState.phase = 'synchronizing';
      }
      if (action.type === 'INVALIDATION_RECEIVED') {
        currentState.pendingRefetch = true;
      }
      if (action.type === 'CLEAR_PENDING_REFETCH') {
        currentState.pendingRefetch = false;
      }
    });

    getState = vi.fn().mockImplementation(() => currentState);
    synchronizer = new JobStoreSynchronizer(dispatch as any, getState as any);
  });

  it('buffers event received after listeners registered but before snapshot completes', async () => {
    let eventCallback: any;
    (subscribeJobEvents as any).mockImplementation((cb: any) => {
      eventCallback = cb;
      return Promise.resolve(vi.fn());
    });
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    let resolveSnapshot: any;
    (getJobsSnapshot as any).mockImplementation(() => new Promise((r) => { resolveSnapshot = r; }));

    const startPromise = synchronizer.startCycle('p1');

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    const event = createEvent(createJob('j1', 1));
    eventCallback(event);

    expect(dispatch).toHaveBeenCalledWith({ type: 'EVENT_RECEIVED', event, generation: 1 });

    await act(async () => {
      resolveSnapshot([]);
      await startPromise;
    });
  });

  it('schedules exactly one next fetch for multiple invalidations during active fetch', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    let resolveFetch: any;
    (getJobsSnapshot as any).mockImplementation(() => new Promise((r) => { resolveFetch = r; }));

    await synchronizer.startCycle('p1');
    expect(getJobsSnapshot).toHaveBeenCalledTimes(1);

    synchronizer.requestFetch(1);
    synchronizer.requestFetch(1);
    synchronizer.requestFetch(1);

    await act(async () => {
      resolveFetch([]);
      await vi.runAllTimersAsync();
    });

    expect(getJobsSnapshot).toHaveBeenCalledTimes(2);
  });

  it('schedules another fetch for invalidation during follow-up fetch', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    let resolveFetch1: any, resolveFetch2: any;
    (getJobsSnapshot as any)
      .mockImplementationOnce(() => new Promise((r) => { resolveFetch1 = r; }))
      .mockImplementationOnce(() => new Promise((r) => { resolveFetch2 = r; }));

    await synchronizer.startCycle('p1');

    synchronizer.requestFetch(1);

    await act(async () => {
      resolveFetch1([]);
      await vi.runAllTimersAsync();
    });

    expect(getJobsSnapshot).toHaveBeenCalledTimes(2);

    synchronizer.requestFetch(1);

    await act(async () => {
      resolveFetch2([]);
      await vi.runAllTimersAsync();
    });

    expect(getJobsSnapshot).toHaveBeenCalledTimes(3);
  });

  it('ignores snapshot resolution of old generation after project switch', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    let resolveSnapshotProj1: any;
    (getJobsSnapshot as any).mockImplementationOnce(() => new Promise((r) => { resolveSnapshotProj1 = r; }));

    await synchronizer.startCycle('p1');

    (getJobsSnapshot as any).mockResolvedValue([]);
    await synchronizer.startCycle('p2');

    await act(async () => {
      resolveSnapshotProj1([]);
      await vi.runAllTimersAsync();
    });

    expect(dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ type: 'SNAPSHOT_RESOLVED', generation: 1 }));
  });

  it('ignores snapshot resolution after dispose', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    let resolveSnapshot: any;
    (getJobsSnapshot as any).mockImplementation(() => new Promise((r) => { resolveSnapshot = r; }));

    await synchronizer.startCycle('p1');
    synchronizer.dispose();

    await act(async () => {
      resolveSnapshot([]);
      await vi.runAllTimersAsync();
    });

    expect(dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ type: 'SNAPSHOT_RESOLVED' }));
  });
});

async function act(callback: () => Promise<void> | void) {
  await callback();
}
