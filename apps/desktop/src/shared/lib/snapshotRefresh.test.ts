// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { subscribeSnapshotRefresh } from './snapshotRefresh';
afterEach(() => vi.useRealTimers());
it('recovers missed events without overlapping or leaking refreshes', async () => {
  vi.useFakeTimers();
  let finish!: () => void;
  const refresh = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  const dispose = subscribeSnapshotRefresh(refresh);
  expect(refresh).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(30_000);
  expect(refresh).toHaveBeenCalledTimes(1);
  window.dispatchEvent(new Event('focus'));
  await vi.advanceTimersByTimeAsync(90_000);
  expect(refresh).toHaveBeenCalledTimes(1);
  finish();
  await vi.advanceTimersByTimeAsync(30_000);
  expect(refresh).toHaveBeenCalledTimes(2);
  dispose();
  finish();
  await vi.advanceTimersByTimeAsync(60_000);
  window.dispatchEvent(new Event('focus'));
  expect(refresh).toHaveBeenCalledTimes(2);
});
