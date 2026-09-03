import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { JobStoreSynchronizer } from '../synchronization';
import { initializeStore, jobStoreReducer, type JobStoreAction } from '../reducer';
import { listJobs, subscribeJobEvents, subscribeJobsInvalidated } from '../../api/jobApi';
import type { JobDto, JobEventDto, JobStoreState } from '../types';

vi.mock('../../api/jobApi', () => ({
  subscribeJobEvents: vi.fn(),
  subscribeJobsInvalidated: vi.fn(),
  listJobs: vi.fn(),
}));

describe('JobStoreSynchronizer - Core', () => {
  let dispatch: ReturnType<typeof vi.fn<(action: JobStoreAction) => void>>;
  let synchronizer: JobStoreSynchronizer;
  let currentState: JobStoreState;

  const createJob = (id: string, revision: number, projectId: string | null = 'p1'): JobDto => ({
    kind: 'dubbing',
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
    vi.resetAllMocks();

    currentState = initializeStore();
    dispatch = vi.fn((action: JobStoreAction) => {
      currentState = jobStoreReducer(currentState, action);
    });
    synchronizer = new JobStoreSynchronizer(dispatch);
  });

  afterEach(() => {
    synchronizer.dispose();
    vi.useRealTimers();
  });

  it('registers listeners and executes initial snapshot fetch in sequence', async () => {
    let resolveEvents: any, resolveInvalidated: any, resolveSnapshot: any;
    const order: string[] = [];

    (subscribeJobEvents as any).mockImplementation(() => {
      order.push('subscribeJobEvents');
      return new Promise((r) => {
        resolveEvents = r;
      });
    });
    (subscribeJobsInvalidated as any).mockImplementation(() => {
      order.push('subscribeJobsInvalidated');
      return new Promise((r) => {
        resolveInvalidated = r;
      });
    });
    (listJobs as any).mockImplementation(() => {
      order.push('listJobs');
      return new Promise((r) => {
        resolveSnapshot = r;
      });
    });

    const promise = synchronizer.startCycle();

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

    expect(order).toEqual(['subscribeJobEvents', 'subscribeJobsInvalidated', 'listJobs']);
  });

  it('rejects duplicate IDs across projects and preserves the last valid global state', async () => {
    vi.mocked(subscribeJobEvents).mockResolvedValue(vi.fn());
    vi.mocked(subscribeJobsInvalidated).mockResolvedValue(vi.fn());
    const jobs = [createJob('j1', 1), createJob('j2', 1, 'p2'), createJob('j3', 1, null)];
    vi.mocked(listJobs).mockResolvedValue(jobs);
    await synchronizer.startCycle();
    expect(Object.values(currentState.jobs)).toEqual(jobs);
    vi.mocked(listJobs).mockResolvedValue([jobs[0], { ...jobs[1], id: 'j1' }]);
    synchronizer.requestFetch(1);
    await Promise.resolve();
    expect(currentState.phase).toBe('stale');
    expect(Object.values(currentState.jobs)).toEqual(jobs);
    vi.mocked(listJobs).mockResolvedValue(jobs);
    await vi.runAllTimersAsync();
    expect(currentState.phase).toBe('ready');
  });

  it('rejects unknown event kinds and logs only generic static message without payload', async () => {
    const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    let eventCallback: any;

    (subscribeJobEvents as any).mockImplementation((cb: any) => {
      eventCallback = cb;
      return Promise.resolve(vi.fn());
    });
    (subscribeJobsInvalidated as any).mockImplementation(() => Promise.resolve(vi.fn()));
    (listJobs as any).mockResolvedValue([]);

    await synchronizer.startCycle();

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

    await synchronizer.startCycle();

    expect(unlistenFirst).toHaveBeenCalledTimes(1);
    expect(dispatch).toHaveBeenCalledWith({ type: 'LISTENERS_FAILED', generation: 1 });
  });

  it('does not dispatch if dispose is called during listener registration', async () => {
    let resolveEvents: any;
    (subscribeJobEvents as any).mockImplementation(
      () =>
        new Promise((r) => {
          resolveEvents = r;
        }),
    );
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    const promise = synchronizer.startCycle();
    synchronizer.dispose();

    await act(async () => {
      resolveEvents(vi.fn());
      await promise;
    });

    expect(dispatch).not.toHaveBeenCalledWith(
      expect.objectContaining({ type: 'LISTENERS_REGISTERED' }),
    );
  });

  it('sets pendingFetch = true and defers fetch if requestFetch is called before listeners are ready', async () => {
    let resolveEvents: any;
    (subscribeJobEvents as any).mockImplementation(
      () =>
        new Promise((r) => {
          resolveEvents = r;
        }),
    );
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (listJobs as any).mockResolvedValue([]);

    const promise = synchronizer.startCycle();
    synchronizer.requestFetch(1);

    expect(listJobs).not.toHaveBeenCalled();

    await act(async () => {
      resolveEvents(vi.fn());
      await promise;
    });

    expect(listJobs).toHaveBeenCalledTimes(1);
  });

  it('waits for retry backoff on snapshot failure and preserves pendingFetch', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (listJobs as any).mockRejectedValueOnce(new Error('Snapshot failed'));

    await synchronizer.startCycle();
    expect(dispatch).toHaveBeenCalledWith({ type: 'FETCH_FAILED', generation: 1 });

    synchronizer.requestFetch(1);

    expect(listJobs).toHaveBeenCalledTimes(1);
    (listJobs as any).mockResolvedValue([]);
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(listJobs).toHaveBeenCalledTimes(2);
  });

  it('ignores old listener events and invalidation callbacks after synchronization restarts', async () => {
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
    (listJobs as any).mockResolvedValue([]);

    await synchronizer.startCycle();

    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    await synchronizer.startCycle();

    eventCallback1(createEvent(createJob('j1', 1)));
    invalidationCallback1();

    expect(dispatch).not.toHaveBeenCalledWith(
      expect.objectContaining({ generation: 1, type: 'EVENT_RECEIVED' }),
    );
    expect(dispatch).not.toHaveBeenCalledWith(
      expect.objectContaining({ generation: 1, type: 'INVALIDATION_RECEIVED' }),
    );
  });

  it('guarantees at most one active snapshot fetch request inside the same generation', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());

    let resolveFetch: any;
    (listJobs as any).mockImplementation(
      () =>
        new Promise((r) => {
          resolveFetch = r;
        }),
    );

    await synchronizer.startCycle();
    expect(listJobs).toHaveBeenCalledTimes(1);

    synchronizer.requestFetch(1);
    synchronizer.requestFetch(1);

    expect(listJobs).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveFetch([]);
      await vi.runAllTimersAsync();
    });

    expect(listJobs).toHaveBeenCalledTimes(2);
  });

  it('runs infinite capped retries for listener failures and resets on success', async () => {
    (subscribeJobEvents as any).mockRejectedValue(new Error('Failed'));

    await synchronizer.startCycle();
    expect(dispatch).toHaveBeenLastCalledWith({ type: 'LISTENERS_FAILED', generation: 1 });

    for (let i = 0; i < 10; i++) {
      await act(async () => {
        await vi.runOnlyPendingTimersAsync();
      });
    }

    expect(subscribeJobEvents).toHaveBeenCalledTimes(11);

    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (listJobs as any).mockResolvedValue([]);

    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });

    expect(dispatch).toHaveBeenCalledWith({ type: 'LISTENERS_REGISTERED', generation: 1 });
  });

  it('resets retry attempt counters on successful snapshot resolution', async () => {
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (listJobs as any).mockRejectedValueOnce(new Error('Fail'));

    await synchronizer.startCycle();
    expect(dispatch).toHaveBeenCalledWith({ type: 'FETCH_FAILED', generation: 1 });

    (listJobs as any).mockResolvedValue([]);
    await act(async () => {
      await vi.runAllTimersAsync();
    });

    expect(dispatch).toHaveBeenLastCalledWith(
      expect.objectContaining({ type: 'SNAPSHOT_RESOLVED', generation: 1 }),
    );
  });

  it('protects against raw leakage in console.error when snapshot fetch fails', async () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    (subscribeJobEvents as any).mockResolvedValue(vi.fn());
    (subscribeJobsInvalidated as any).mockResolvedValue(vi.fn());
    (listJobs as any).mockRejectedValue(new Error('C:\\Users\\secret\\video.mp4 token=SECRET'));
    await synchronizer.startCycle();
    expect(dispatch).toHaveBeenCalledWith({ type: 'FETCH_FAILED', generation: 1 });
    spy.mock.calls.forEach((call) => {
      const log = JSON.stringify(call);
      ['secret', 'SECRET', 'token', 'video.mp4'].forEach((s) => expect(log).not.toContain(s));
    });
    spy.mockRestore();
  });
});

async function act(callback: () => Promise<void> | void) {
  await callback();
}
