import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useWindowChrome } from './useWindowChrome';
import { deferred } from '../../../../tests/studio';

const native = vi.hoisted(() => ({
  isTauri: vi.fn(),
  getCurrentWindow: vi.fn(),
  isMaximized: vi.fn(),
  onResized: vi.fn(),
  minimize: vi.fn(),
  toggleMaximize: vi.fn(),
  close: vi.fn(),
  stop: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: native.isTauri }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: native.getCurrentWindow }));
beforeEach(() => {
  vi.resetAllMocks();
  native.isTauri.mockReturnValue(true);
  native.getCurrentWindow.mockReturnValue(native);
  native.isMaximized.mockResolvedValue(false);
  native.onResized.mockResolvedValue(native.stop);
});
afterEach(cleanup);
it('reports an unavailable native window handle without crashing the workspace', () => {
  native.getCurrentWindow.mockImplementation(() => {
    throw new Error('Window handle unavailable');
  });
  const { result } = renderHook(useWindowChrome);
  expect(result.current.available).toBe(false);
  expect(result.current.error).toContain('Window handle unavailable');
});
it('admits a repeated window operation once while the native response is pending', async () => {
  const pending = deferred<void>();
  native.minimize.mockReturnValue(pending.promise);
  const { result } = renderHook(useWindowChrome);
  let first!: Promise<void>;
  let second!: Promise<void>;
  act(() => {
    first = result.current.run('minimize');
    second = result.current.run('minimize');
  });
  expect(native.minimize).toHaveBeenCalledTimes(1);
  await act(async () => {
    pending.resolve();
    await Promise.all([first, second]);
  });
});
it('does not replace the latest maximize state with an older resize reply', async () => {
  const old = deferred<boolean>();
  native.isMaximized.mockReturnValueOnce(old.promise).mockResolvedValue(true);
  const { result } = renderHook(useWindowChrome);
  await act(async () => native.onResized.mock.calls[0][0]());
  expect(result.current.maximized).toBe(true);
  await act(async () => old.resolve(false));
  expect(result.current.maximized).toBe(true);
});
it('ignores a late native failure after unmount and still releases its listener', async () => {
  const pending = deferred<void>();
  native.close.mockReturnValue(pending.promise);
  const { result, unmount } = renderHook(useWindowChrome);
  let closing!: Promise<void>;
  act(() => {
    closing = result.current.run('close');
  });
  unmount();
  await act(async () => {
    pending.reject(new Error('Late failure'));
    await closing;
  });
  expect(native.stop).toHaveBeenCalledTimes(1);
});
it('does not use native window APIs in a browser', async () => {
  native.isTauri.mockReturnValue(false);
  const { result } = renderHook(useWindowChrome);
  expect(result.current.available).toBe(false);
  await act(() => result.current.run('close'));
  expect(native.getCurrentWindow).not.toHaveBeenCalled();
  expect(native.close).not.toHaveBeenCalled();
});
it('synchronizes maximize state on resize and releases the listener', async () => {
  const { result, unmount } = renderHook(useWindowChrome);
  await waitFor(() => expect(native.onResized).toHaveBeenCalledTimes(1));
  native.isMaximized.mockResolvedValue(true);
  await act(async () => native.onResized.mock.calls[0][0]());
  expect(result.current.maximized).toBe(true);
  unmount();
  await waitFor(() => expect(native.stop).toHaveBeenCalledTimes(1));
});
it('releases a subscription that arrives after unmount', async () => {
  const subscription = deferred<() => void>();
  native.onResized.mockReturnValue(subscription.promise);
  const { unmount } = renderHook(useWindowChrome);
  unmount();
  await act(async () => subscription.resolve(native.stop));
  expect(native.stop).toHaveBeenCalledTimes(1);
});
it.each(['isMaximized', 'onResized'] as const)('shows %s failures', async (method) => {
  native[method].mockRejectedValue(new Error('Window unavailable'));
  const { result } = renderHook(useWindowChrome);
  await waitFor(() => expect(result.current.error).toContain('Window unavailable'));
});
it.each(['minimize', 'toggleMaximize', 'close'] as const)(
  'reports and recovers from failed %s',
  async (method) => {
    native[method].mockRejectedValueOnce(new Error('Native denied')).mockResolvedValue(undefined);
    const { result } = renderHook(useWindowChrome);
    await act(() => result.current.run(method));
    expect(result.current.error).toContain('Native denied');
    native.isMaximized.mockResolvedValue(true);
    await act(() => result.current.run(method));
    expect(result.current.error).toBe('');
    expect(native[method]).toHaveBeenCalledTimes(2);
    if (method === 'toggleMaximize') expect(result.current.maximized).toBe(true);
  },
);
