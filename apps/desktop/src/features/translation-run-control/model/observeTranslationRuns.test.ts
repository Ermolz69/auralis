import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { observeTranslationRuns } from './observeTranslationRuns';

const api = vi.hoisted(() => ({
  getProjectTranslationSnapshot: vi.fn(),
  subscribeTranslationChanges: vi.fn(),
}));
vi.mock('@/entities/translation', () => api);

const link = { projectId: 'project', translationId: 'translation', activeRunId: 'run' };
const run = { runId: 'run', phase: 'running', savedBlocks: 0, totalBlocks: 2 };
const snapshot = (links = [link], runs = [run]) => ({
  projectId: 'project',
  entries: links.map((item, index) => ({ link: item, run: runs[index] ?? null })),
});
let notify: () => void;
let stop: () => void;
const dispose = vi.fn();
const onSnapshot = vi.fn();
const onError = vi.fn();

async function settle() {
  for (let index = 0; index < 12; index++) await Promise.resolve();
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.resetAllMocks();
  api.getProjectTranslationSnapshot.mockResolvedValue(snapshot());
  api.subscribeTranslationChanges.mockImplementation(async (_projectId, refresh) => {
    notify = refresh;
    return dispose;
  });
});
afterEach(() => {
  stop?.();
  vi.useRealTimers();
});

describe('translation run observation', () => {
  it('uses one project IPC read for empty, one-run and four-run snapshots', async () => {
    for (const count of [0, 1, 4]) {
      const links = Array.from({ length: count }, (_, index) => ({
        ...link,
        translationId: `translation-${index}`,
        activeRunId: `run-${index}`,
      }));
      const runs = links.map((item) => ({ ...run, runId: item.activeRunId }));
      api.getProjectTranslationSnapshot.mockResolvedValue(snapshot(links, runs));
      const before = api.getProjectTranslationSnapshot.mock.calls.length;
      stop = observeTranslationRuns('project', { onSnapshot, onError });
      await settle();
      expect(api.getProjectTranslationSnapshot).toHaveBeenCalledTimes(before + 1);
      expect(onSnapshot).toHaveBeenLastCalledWith(
        links,
        Object.fromEntries(links.map((item, index) => [item.translationId, runs[index]])),
      );
      stop();
    }
  });

  it('refreshes committed counts on an event and keeps three-second recovery', async () => {
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    api.getProjectTranslationSnapshot.mockResolvedValue(
      snapshot([link], [{ ...run, savedBlocks: 1 }]),
    );
    notify();
    await settle();
    expect(onSnapshot).toHaveBeenLastCalledWith([link], {
      translation: { ...run, savedBlocks: 1 },
    });
    expect(api.getProjectTranslationSnapshot).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(3000);
    expect(api.getProjectTranslationSnapshot).toHaveBeenCalledTimes(3);
  });

  it('discards an in-flight answer and coalesces twenty events into one trailing read', async () => {
    let resolve: (value: unknown) => void = () => {};
    api.getProjectTranslationSnapshot.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    for (let index = 0; index < 20; index++) notify();
    api.getProjectTranslationSnapshot.mockResolvedValue(
      snapshot([link], [{ ...run, savedBlocks: 1 }]),
    );
    resolve(snapshot());
    await settle();
    expect(api.getProjectTranslationSnapshot).toHaveBeenCalledTimes(2);
    expect(onSnapshot).toHaveBeenCalledTimes(1);
    expect(onSnapshot).toHaveBeenLastCalledWith([link], {
      translation: { ...run, savedBlocks: 1 },
    });
  });

  it('discards a stale read failure when a newer event is waiting', async () => {
    let reject: (reason: unknown) => void = () => {};
    api.getProjectTranslationSnapshot.mockImplementationOnce(
      () =>
        new Promise((_done, fail) => {
          reject = fail;
        }),
    );
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    notify();
    api.getProjectTranslationSnapshot.mockResolvedValue(snapshot());
    reject(new Error('stale failure'));
    await settle();
    expect(api.getProjectTranslationSnapshot).toHaveBeenCalledTimes(2);
    expect(onError).not.toHaveBeenCalled();
    expect(onSnapshot).toHaveBeenCalledTimes(1);
  });

  it('recovers a lost event, pause, resume and completion from committed reads', async () => {
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    for (const phase of ['paused', 'running', 'validated']) {
      const next = { ...run, phase, savedBlocks: phase === 'validated' ? 2 : 1 };
      api.getProjectTranslationSnapshot.mockResolvedValue(snapshot([link], [next]));
      await vi.advanceTimersByTimeAsync(3000);
      expect(onSnapshot).toHaveBeenLastCalledWith([link], { translation: next });
    }
    expect(api.getProjectTranslationSnapshot).toHaveBeenCalledTimes(4);
  });

  it('ignores reads and callbacks from a disposed project generation', async () => {
    let resolve: (value: unknown) => void = () => {};
    api.getProjectTranslationSnapshot.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    stop = observeTranslationRuns('old', { onSnapshot, onError });
    await settle();
    const oldNotify = notify;
    stop();
    api.getProjectTranslationSnapshot.mockResolvedValue(snapshot());
    stop = observeTranslationRuns('new', { onSnapshot, onError });
    await settle();
    resolve(snapshot([link], [{ ...run, savedBlocks: 99 }]));
    oldNotify();
    await settle();
    expect(onSnapshot).toHaveBeenCalledTimes(1);
    expect(dispose).toHaveBeenCalledTimes(1);
  });

  it('recovers after a failed read and unavailable subscription', async () => {
    api.subscribeTranslationChanges.mockRejectedValue(new Error('No events'));
    api.getProjectTranslationSnapshot.mockRejectedValueOnce(new Error('Read unavailable'));
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    expect(onError).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(3000);
    expect(onSnapshot).toHaveBeenCalledWith([link], { translation: run });
  });

  it('polls during stalled registration and disposes late listeners', async () => {
    let registered: (value: () => void) => void = () => {};
    api.subscribeTranslationChanges.mockImplementation(
      () =>
        new Promise((done) => {
          registered = done;
        }),
    );
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await vi.advanceTimersByTimeAsync(3000);
    expect(onSnapshot).toHaveBeenCalledTimes(1);
    registered(dispose);
    await settle();
    expect(onSnapshot).toHaveBeenCalledTimes(2);
    stop();
    stop = observeTranslationRuns('next', { onSnapshot, onError });
    stop();
    registered(dispose);
    await settle();
    expect(dispose).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(6000);
    expect(onSnapshot).toHaveBeenCalledTimes(2);
  });
});
