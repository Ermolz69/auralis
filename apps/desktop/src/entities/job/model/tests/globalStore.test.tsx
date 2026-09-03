// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render, renderHook, waitFor } from '@testing-library/react';
import { StrictMode, type ReactNode } from 'react';
import { JobProvider } from '../JobProvider';
import { useJobContext, useProjectJobs } from '../useJobContext';
import { listJobs, subscribeJobEvents, subscribeJobsInvalidated } from '../../api/jobApi';
import type { JobDto, JobEventDto } from '../types';

vi.mock('../../api/jobApi', () => ({
  listJobs: vi.fn(),
  subscribeJobEvents: vi.fn(),
  subscribeJobsInvalidated: vi.fn(),
}));

const makeJob = (id: string, projectId: string | null): JobDto => ({
  kind: 'dubbing',
  id,
  projectId,
  revision: 1,
  title: id,
  status: 'running',
  stage: null,
  progress: {
    percent: 10,
    message: 'Working',
    currentStep: null,
    processedItems: null,
    totalItems: null,
  },
  error: null,
  createdAt: '',
  updatedAt: '',
});
const first = makeJob('first', 'p1');
const second = makeJob('second', 'p2');
const unattached = makeJob('unattached', null);
let onEvent: (event: JobEventDto) => void;
let onInvalidation: () => void;
let unlistenEvents: ReturnType<typeof vi.fn<() => void>>;
let unlistenInvalidations: ReturnType<typeof vi.fn<() => void>>;

beforeEach(() => {
  vi.resetAllMocks();
  unlistenEvents = vi.fn();
  unlistenInvalidations = vi.fn();
  vi.mocked(listJobs).mockResolvedValue([first, second, unattached]);
  vi.mocked(subscribeJobEvents).mockImplementation(async (handler) => {
    onEvent = handler;
    return unlistenEvents;
  });
  vi.mocked(subscribeJobsInvalidated).mockImplementation(async (handler) => {
    onInvalidation = handler;
    return unlistenInvalidations;
  });
});
afterEach(cleanup);

function renderStore(initialProjectId: string | null = null) {
  return renderHook(
    ({ projectId }: { projectId: string | null }) => ({
      global: useJobContext(),
      scoped: useProjectJobs(projectId),
    }),
    {
      initialProps: { projectId: initialProjectId },
      wrapper: ({ children }: { children: ReactNode }) => <JobProvider>{children}</JobProvider>,
    },
  );
}

describe('global job store', () => {
  it('loads without a project and changes selectors without restarting synchronization', async () => {
    const { result, rerender, unmount } = renderStore();
    await waitFor(() => expect(result.current.global.phase).toBe('ready'));
    expect(result.current.global.activeJobs).toEqual([first, second, unattached]);
    expect(result.current.scoped.jobs).toEqual([]);
    for (const [projectId, expected] of [
      ['p1', first],
      ['p2', second],
    ] as const) {
      rerender({ projectId });
      expect(result.current.scoped.jobs).toEqual([expected]);
      expect(result.current.scoped.activeJobs).toEqual([expected]);
      expect(result.current.global.activeJobs).toEqual([first, second, unattached]);
    }
    rerender({ projectId: null });
    expect(result.current.scoped.jobs).toEqual([]);
    expect(result.current.global.activeJobs).toHaveLength(3);
    expect(listJobs).toHaveBeenCalledExactlyOnceWith();
    expect(subscribeJobEvents).toHaveBeenCalledTimes(1);
    expect(subscribeJobsInvalidated).toHaveBeenCalledTimes(1);
    expect(unlistenEvents).not.toHaveBeenCalled();
    expect(unlistenInvalidations).not.toHaveBeenCalled();
    unmount();
    expect(unlistenEvents).toHaveBeenCalledTimes(1);
    expect(unlistenInvalidations).toHaveBeenCalledTimes(1);
  });

  it('receives progress and completion for other projects and unattached jobs', async () => {
    const { result } = renderStore('p2');
    await waitFor(() => expect(result.current.global.phase).toBe('ready'));
    const completed = { ...first, revision: 2, status: 'completed' as const };
    const progressed = {
      ...unattached,
      revision: 2,
      progress: { ...unattached.progress, percent: 60 },
    };
    act(() => {
      onEvent({ kind: 'completed', job: completed });
      onEvent({ kind: 'progressed', job: progressed });
    });
    expect(result.current.global.completedJobs).toEqual([completed]);
    expect(result.current.global.activeJobs).toEqual([second, progressed]);
    expect(result.current.scoped.jobs).toEqual([second]);
    expect(result.current.scoped.completedJobs).toEqual([]);
    expect(listJobs).toHaveBeenCalledTimes(1);
  });

  it('replays events from all projects over a delayed refresh without losing progress', async () => {
    const { result } = renderStore('p2');
    await waitFor(() => expect(result.current.global.phase).toBe('ready'));
    let resolve!: (jobs: JobDto[]) => void;
    vi.mocked(listJobs).mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    act(() => onInvalidation());
    const completed = { ...first, revision: 2, status: 'completed' as const };
    const created = makeJob('third', 'p3');
    act(() => {
      onEvent({ kind: 'completed', job: completed });
      onEvent({ kind: 'created', job: created });
    });
    await act(async () => resolve([first, second, unattached]));
    expect(result.current.global.phase).toBe('ready');
    expect(result.current.global.jobs.first).toEqual(completed);
    expect(result.current.global.jobs.third).toEqual(created);
    expect(result.current.scoped.jobs).toEqual([second]);
  });

  it('recovers revision gaps in another project through the global snapshot', async () => {
    const { result } = renderStore('p2');
    await waitFor(() => expect(result.current.global.phase).toBe('ready'));
    const completed = { ...first, revision: 4, status: 'completed' as const };
    vi.mocked(listJobs).mockResolvedValue([completed, second]);
    await act(async () => onEvent({ kind: 'completed', job: completed }));
    await waitFor(() => expect(result.current.global.completedJobs).toEqual([completed]));
    expect(listJobs).toHaveBeenCalledTimes(2);
    expect(result.current.scoped.activeJobs).toEqual([second]);
  });

  it('survives StrictMode effect replay and cleans up subscriptions', async () => {
    function Probe() {
      const { activeJobs } = useJobContext();
      return <output>{activeJobs.length}</output>;
    }
    const { unmount, getByText } = render(
      <StrictMode>
        <JobProvider>
          <Probe />
        </JobProvider>
      </StrictMode>,
    );
    await waitFor(() => expect(getByText('3')).toBeTruthy());
    expect(subscribeJobEvents).toHaveBeenCalledTimes(2);
    expect(listJobs).toHaveBeenCalledTimes(1);
    unmount();
    expect(unlistenEvents).toHaveBeenCalledTimes(2);
    expect(unlistenInvalidations).toHaveBeenCalledTimes(1);
  });
});
