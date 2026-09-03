import { describe, it, expect } from 'vitest';
import { jobStoreReducer, initializeStore } from '../reducer';
import type { JobDto, JobEventDto, JobStoreState } from '../types';

describe('jobStoreReducer', () => {
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

  it('generation-starting actions (INITIALIZATION_CYCLE) reject equal or smaller generation', () => {
    const state = initializeStore();
    state.generation = 5;

    const action = { type: 'INITIALIZATION_CYCLE' as const, generation: 5 };
    const nextState = jobStoreReducer(state, action);
    expect(nextState).toBe(state); // ignored

    const actionNewer = { type: 'INITIALIZATION_CYCLE' as const, generation: 6 };
    const nextStateNewer = jobStoreReducer(state, actionNewer);
    expect(nextStateNewer.generation).toBe(6);
    expect(nextStateNewer.phase).toBe('initializing');
  });

  it('INITIALIZATION_CYCLE clears buffer and pendingRefetch but preserves existing jobs', () => {
    const state: JobStoreState = {
      phase: 'ready',
      jobs: {
        j1: createJob('j1', 1),
      },
      buffer: [createEvent(createJob('j2', 1))],
      pendingRefetch: true,
      generation: 5,
    };

    const action = { type: 'INITIALIZATION_CYCLE' as const, generation: 6 };
    const nextState = jobStoreReducer(state, action);
    expect(nextState.phase).toBe('initializing');
    expect(nextState.generation).toBe(6);
    expect(nextState.buffer).toEqual([]);
    expect(nextState.pendingRefetch).toBe(false);
    expect(nextState.jobs).toEqual(state.jobs); // preserved!
  });

  it('cycle-scoped actions check and enforce exact matching generation', () => {
    const state: JobStoreState = {
      phase: 'ready',
      jobs: {},
      buffer: [],
      pendingRefetch: false,
      generation: 5,
    };

    // Action with mismatched generation (6 !== 5)
    const actionMismatched = {
      type: 'EVENT_RECEIVED' as const,
      event: createEvent(createJob('j1', 1)),
      generation: 6,
    };
    const nextState = jobStoreReducer(state, actionMismatched);
    expect(nextState).toBe(state); // ignored

    // Action with matching generation (5 === 5)
    const actionMatched = {
      type: 'EVENT_RECEIVED' as const,
      event: createEvent(createJob('j1', 1)),
      generation: 5,
    };
    const nextStateMatched = jobStoreReducer(state, actionMatched);
    expect(nextStateMatched.jobs['j1']).toBeDefined();
  });

  it('reconciliation ignores older or equal revisions', () => {
    const jobV2 = createJob('j1', 2);
    const state: JobStoreState = {
      phase: 'ready',
      jobs: {
        j1: jobV2,
      },
      buffer: [],
      pendingRefetch: false,
      generation: 1,
    };

    // Older revision (1 < 2)
    const eventV1 = createEvent(createJob('j1', 1));
    const state1 = jobStoreReducer(state, {
      type: 'EVENT_RECEIVED',
      event: eventV1,
      generation: 1,
    });
    expect(state1.jobs['j1'].revision).toBe(2);

    // Equal revision (2 === 2)
    const eventV2 = createEvent(createJob('j1', 2));
    const state2 = jobStoreReducer(state, {
      type: 'EVENT_RECEIVED',
      event: eventV2,
      generation: 1,
    });
    expect(state2.jobs['j1'].revision).toBe(2);
  });

  it('reconciliation applies consecutive revision (current + 1)', () => {
    const jobV1 = createJob('j1', 1);
    const state: JobStoreState = {
      phase: 'ready',
      jobs: {
        j1: jobV1,
      },
      buffer: [],
      pendingRefetch: false,
      generation: 1,
    };

    const eventV2 = createEvent(createJob('j1', 2));
    const nextState = jobStoreReducer(state, {
      type: 'EVENT_RECEIVED',
      event: eventV2,
      generation: 1,
    });
    expect(nextState.jobs['j1'].revision).toBe(2);
  });

  it('reconciliation detects revision gap, transitions to stale, and sets pendingRefetch', () => {
    const jobV1 = createJob('j1', 1);
    const state: JobStoreState = {
      phase: 'ready',
      jobs: {
        j1: jobV1,
      },
      buffer: [],
      pendingRefetch: false,
      generation: 1,
    };

    // Gap (3 > 1 + 1)
    const eventV3 = createEvent(createJob('j1', 3));
    const nextState = jobStoreReducer(state, {
      type: 'EVENT_RECEIVED',
      event: eventV3,
      generation: 1,
    });
    expect(nextState.phase).toBe('stale');
    expect(nextState.pendingRefetch).toBe(true);
  });

  it('buffer overflow clears buffer, sets state to stale, and sets pendingRefetch', () => {
    const state: JobStoreState = {
      phase: 'synchronizing',
      jobs: {},
      buffer: Array.from({ length: 256 }, (_, i) => createEvent(createJob(`j${i}`, 1))),
      pendingRefetch: false,
      generation: 1,
    };

    // The 257th event causes overflow
    const event = createEvent(createJob('overflow-job', 1));
    const nextState = jobStoreReducer(state, { type: 'EVENT_RECEIVED', event, generation: 1 });

    expect(nextState.phase).toBe('stale');
    expect(nextState.buffer).toEqual([]);
    expect(nextState.pendingRefetch).toBe(true);
  });

  it('keeps newer known revisions when a background snapshot is older', () => {
    const latest = createJob('j1', 5);
    const state: JobStoreState = {
      ...initializeStore(),
      phase: 'ready',
      jobs: { j1: latest },
    };
    const fetching = jobStoreReducer(state, { type: 'FETCH_STARTED', generation: 0 });
    expect(fetching.phase).toBe('synchronizing');
    const next = jobStoreReducer(fetching, {
      type: 'SNAPSHOT_RESOLVED',
      generation: 0,
      jobs: [createJob('j1', 4)],
    });
    expect(next.jobs.j1).toBe(latest);
  });

  it('replays events for other projects even when one buffered job has a revision gap', () => {
    const other = createJob('j2', 2, 'p2');
    const unattached = createJob('j3', 1, null);
    const state: JobStoreState = {
      ...initializeStore(),
      phase: 'synchronizing',
      buffer: [createEvent(createJob('j1', 4)), createEvent(other), createEvent(unattached)],
    };
    const next = jobStoreReducer(state, {
      type: 'SNAPSHOT_RESOLVED',
      generation: 0,
      jobs: [createJob('j1', 1), createJob('j2', 1, 'p2')],
    });
    expect(next.pendingRefetch).toBe(true);
    expect(next.phase).toBe('stale');
    expect(next.jobs.j2).toBe(other);
    expect(next.jobs.j3).toBe(unattached);
  });

  it('stale-generation event does not cause buffer overflow of a new cycle', () => {
    const state: JobStoreState = {
      phase: 'synchronizing',
      jobs: {},
      buffer: Array.from({ length: 256 }, (_, i) => createEvent(createJob(`j${i}`, 1))),
      pendingRefetch: false,
      generation: 2, // Current generation is 2
    };

    // Stale generation 1 event received
    const event = createEvent(createJob('stale-job', 1));
    const nextState = jobStoreReducer(state, { type: 'EVENT_RECEIVED', event, generation: 1 });

    expect(nextState).toBe(state); // Ignores completely, buffer length remains 256
    expect(nextState.phase).toBe('synchronizing');
    expect(nextState.pendingRefetch).toBe(false);
  });
});
