import { describe, it, expect, vi, beforeEach } from 'vitest';
import { JobStoreSynchronizer } from '../synchronization';
import { getJobsSnapshot, subscribeJobEvents, subscribeJobsInvalidated } from '../../api/jobApi';
import type { JobDto, JobEventDto, JobStoreState } from '../types';

vi.mock('../../api/jobApi', () => ({
  subscribeJobEvents: vi.fn(),
  subscribeJobsInvalidated: vi.fn(),
  getJobsSnapshot: vi.fn(),
}));

describe('JobStoreSynchronizer - Core', () => {
  let dispatch: ReturnType<typeof vi.fn>;
  let getState: ReturnType<typeof vi.fn>;
  let synchronizer: JobStoreSynchronizer;
  let currentState: JobStoreState;

  const createJob = (id: string, revision: number, projectId: string | null = 'p1'): JobDto => ({
    id, revision, projectId, title: `Job ${id}`, status: 'pending', stage: null,
    progress: { percent: 0, message: '', currentStep: null, processedItems: null, totalItems: null },
    error: null, createdAt: '', updatedAt: '',
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
      } else if (action.type === 'INITIALIZATION_CYCLE') {
        currentState.generation = action.generation;
        currentState.phase = 'initializing';
      } else if (action.type === 'LISTENERS_REGISTERED') {
        currentState.phase = 'synchronizing';
      } else if (action.type === 'LISTENERS_FAILED' || action.type === 'FETCH_FAILED') {
        currentState.phase = 'stale';
      } else if (action.type === 'INVALIDATION_RECEIVED') {
        currentState.pendingRefetch = true;
      } else if (action.type === 'CLEAR_PENDING_REFETCH') {
        currentState.pendingRefetch = false;
      }
    });

    getState = vi.fn().mockImplementation(() => currentState);
    synchronizer = new JobStoreSynchronizer(dispatch as any, getState as any);
  });

  it('registers listeners and executes initial snapshot fetch in sequence', async () => {
    let resolveEvents: any, resolveInvalidated: any, resolveSnapshot: any;
    const order: string[] = [];

    (subscribeJobEvents as any).mockImplementation(() => {
      order.push('subscribeJobEvents');
      return new Promise((r) => { resolveEvents = r; });
    });
    (subscribeJobsInvalidated as any).mockImplementation(() => {
      order.push('subscribeJobsInvalidated');
      return new Promise((r) => { resolveInvalidated = r; });
    });
    (getJobsSnapshot as any).mockImplementation(() => {
      order.push('getJobsSnapshot');
      return new Promise((r) => { resolveSnapshot = r; });
    });

    const promise = synchronizer.startCycle('p1');

    await act(async () => {
      resolveEvents(vi.fn());
      await vi.runAllTimersAsync();
    });
    await act(async () => {
      resolveInvalidated(vi.fn());
      await vi.runAllTimersAsync();
    });
    await act(async () => {
      resolveSnapshot([]);
      await promise;
    });

    expect(order).toEqual(['subscribeJobEvents', 'subscribeJobsInvalidated', 'getJobsSnapshot']);
  });

  it('rejects unknown event kinds and logs only generic static message without payload', async () => {
    const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    let eventCallback: any;

    (subscribeJobEvents as any).mockImplementation((cb: any) => {
      eventCallback = cb;
      return Promise.resolve(vi.fn());
    });
    (subscribeJobsInvalidated as any).mockImplementation(() => Promise.resolve(vi.fn()));
    (getJobsSnapshot as any).mockResolvedValue([]);

    await synchronizer.startCycle('p1');

    const invalidEvent = { kind: 'unknown_kind', job: createJob('j1', 1) };
    eventCallback(invalidEvent);

    expect(consoleWarnSpy).toHaveBeenCalledTimes(1);
    expect(consoleWarnSpy.mock.calls[0][0]).toBe('JobStore: Received invalid job event');
    expect(consoleWarnSpy.mock.calls[0].length).toBe(1);

    expect(dispatch).toHaveBeenCalledWith({ type: 'INVALIDATION_RECEIVED', generation: 1 });
    expect(dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ type: 'EVENT_RECEIVED' }));

    consoleWarnSpy.mockRestore();
  });

  it('cleans up the first listener if the second listener registration fails', async () => {
    const unlistenFirst = vi.fn();
    (subscribeJobEvents as any).mockResolvedValue(unlistenFirst);
    (subscribeJobsInvalidated as any).mockRejectedValue(new Error('Failed to register'));

    await synchronizer.startCycle('p1');

    expect(unlistenFirst).toHaveBeenCalledTimes(1);
    expect(dispatch).toHaveBeenCalledWith({ type: 'LISTENERS_FAILED', generation: 1 });
  });

  it('does not dispatch if dispose is called during listener registration', async () => {
    let resolveEvents: any;
    (subscribeJobEvents as any).mockImplementation(() => new Promise((r) => { resolveEvents = r; }));
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    const promise = synchronizer.startCycle('p1');
    synchronizer.dispose();

    await act(async () => {
      resolveEvents(vi.fn());
      await promise;
    });

    expect(dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ type: 'LISTENERS_REGISTERED' }));
  });

  it('sets pendingFetch = true and defers fetch if requestFetch is called before listeners are ready', async () => {
    let resolveEvents: any;
    (subscribeJobEvents as any).mockImplementation(() => new Promise((r) => { resolveEvents = r; }));
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (getJobsSnapshot as any).mockResolvedValue([]);

    const promise = synchronizer.startCycle('p1');
    synchronizer.requestFetch(1);

    expect(getJobsSnapshot).not.toHaveBeenCalled();

    await act(async () => {
      resolveEvents(vi.fn());
      await promise;
    });

    expect(getJobsSnapshot).toHaveBeenCalledTimes(1);
  });

  it('waits for retry backoff on snapshot failure and preserves pendingFetch', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (getJobsSnapshot as any).mockRejectedValueOnce(new Error('Snapshot failed'));

    await synchronizer.startCycle('p1');
    expect(dispatch).toHaveBeenCalledWith({ type: 'FETCH_FAILED', generation: 1 });

    synchronizer.requestFetch(1);

    (getJobsSnapshot as any).mockResolvedValue([]);
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(getJobsSnapshot).toHaveBeenCalledTimes(2);
  });

  it('ignores old listener events and invalidation callbacks after project switch', async () => {
    let eventCallback1: any;
    let invalidationCallback1: any;

    (subscribeJobEvents as any).mockImplementationOnce((cb: any) => {
      eventCallback1 = cb;
      return Promise.resolve(vi.fn());
    });
    (subscribeJobsInvalidated as any).mockImplementationOnce((cb: any) => {
      invalidationCallback1 = cb;
      return Promise.resolve(vi.fn());
    });
    (getJobsSnapshot as any).mockResolvedValue([]);

    await synchronizer.startCycle('p1');

    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    await synchronizer.startCycle('p2');

    eventCallback1(createEvent(createJob('j1', 1)));
    invalidationCallback1();

    expect(dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ generation: 1, type: 'EVENT_RECEIVED' }));
    expect(dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ generation: 1, type: 'INVALIDATION_RECEIVED' }));
  });

  it('guarantees at most one active snapshot fetch request inside the same generation', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    let resolveFetch: any;
    (getJobsSnapshot as any).mockImplementation(() => new Promise((r) => { resolveFetch = r; }));

    await synchronizer.startCycle('p1');
    expect(getJobsSnapshot).toHaveBeenCalledTimes(1);

    synchronizer.requestFetch(1);
    synchronizer.requestFetch(1);

    expect(getJobsSnapshot).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveFetch([]);
      await vi.runAllTimersAsync();
    });

    expect(getJobsSnapshot).toHaveBeenCalledTimes(2);
  });

  it('runs infinite capped retries for listener failures and resets on success', async () => {
    (subscribeJobEvents as any).mockRejectedValue(new Error('Failed'));

    await synchronizer.startCycle('p1');
    expect(dispatch).toHaveBeenLastCalledWith({ type: 'LISTENERS_FAILED', generation: 1 });

    for (let i = 0; i < 10; i++) {
      await act(async () => {
        await vi.runOnlyPendingTimersAsync();
      });
    }

    expect(subscribeJobEvents).toHaveBeenCalledTimes(11);

    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (getJobsSnapshot as any).mockResolvedValue([]);

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    expect(dispatch).toHaveBeenCalledWith({ type: 'LISTENERS_REGISTERED', generation: 1 });
  });

  it('resets retry attempt counters on successful snapshot resolution', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (getJobsSnapshot as any).mockRejectedValueOnce(new Error('Fail'));

    await synchronizer.startCycle('p1');
    expect(dispatch).toHaveBeenCalledWith({ type: 'FETCH_FAILED', generation: 1 });

    (getJobsSnapshot as any).mockResolvedValue([]);
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(dispatch).toHaveBeenLastCalledWith(expect.objectContaining({ type: 'SNAPSHOT_RESOLVED', generation: 1 }));
  });
});

async function act(callback: () => Promise<void> | void) {
  await callback();
}
