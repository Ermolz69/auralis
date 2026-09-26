import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { observeTranslationRuns } from './observeTranslationRuns';

const api = vi.hoisted(() => ({
  listProjectTranslations: vi.fn(),
  getTranslationRunStatus: vi.fn(),
  subscribeTranslationChanges: vi.fn(),
}));
vi.mock('@/entities/translation', () => api);

const link = { translationId: 'translation', activeRunId: 'run' };
const snapshot = { runId: 'run', savedBlocks: 0, totalBlocks: 2 };
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
  api.listProjectTranslations.mockResolvedValue([link]);
  api.getTranslationRunStatus.mockResolvedValue(snapshot);
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
  it('refreshes committed counts on an event without advancing the recovery timer', async () => {
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    api.getTranslationRunStatus.mockResolvedValue({ ...snapshot, savedBlocks: 1 });
    notify();
    await settle();
    expect(onSnapshot).toHaveBeenLastCalledWith([link], {
      translation: { ...snapshot, savedBlocks: 1 },
    });
    expect(api.listProjectTranslations).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(3000);
    expect(api.listProjectTranslations).toHaveBeenCalledTimes(3);
  });

  it('coalesces a burst during a pending read and never runs parallel status reads', async () => {
    let resolve: (value: unknown) => void = () => {};
    api.getTranslationRunStatus.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    notify();
    notify();
    notify();
    expect(api.getTranslationRunStatus).toHaveBeenCalledTimes(1);
    api.getTranslationRunStatus.mockResolvedValue({ ...snapshot, savedBlocks: 1 });
    resolve(snapshot);
    await settle();
    expect(api.getTranslationRunStatus).toHaveBeenCalledTimes(2);
    expect(onSnapshot).toHaveBeenLastCalledWith([link], {
      translation: { ...snapshot, savedBlocks: 1 },
    });
  });

  it('ignores reads and callbacks from a disposed project generation', async () => {
    let resolve: (value: unknown) => void = () => {};
    api.getTranslationRunStatus.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    stop = observeTranslationRuns('old', { onSnapshot, onError });
    await settle();
    const oldNotify = notify;
    stop();
    stop = observeTranslationRuns('new', { onSnapshot, onError });
    await settle();
    resolve({ ...snapshot, savedBlocks: 99 });
    oldNotify();
    await settle();
    expect(onSnapshot).toHaveBeenCalledTimes(1);
    expect(api.listProjectTranslations).toHaveBeenCalledTimes(2);
    expect(dispose).toHaveBeenCalledTimes(1);
  });

  it('recovers after a failed read and unavailable subscription', async () => {
    api.subscribeTranslationChanges.mockRejectedValue(new Error('No events'));
    api.listProjectTranslations.mockRejectedValueOnce(new Error('Read unavailable'));
    stop = observeTranslationRuns('project', { onSnapshot, onError });
    await settle();
    expect(onError).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(3000);
    expect(onSnapshot).toHaveBeenCalledWith([link], { translation: snapshot });
  });

  it('polls during stalled registration, catches up when it resolves, and disposes late listeners', async () => {
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
