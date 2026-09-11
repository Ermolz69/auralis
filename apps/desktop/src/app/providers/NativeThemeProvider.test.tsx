import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { StoredTheme } from '@/shared/api/contracts/uiPreferences';
import { NativeThemeProvider as ThemeProvider } from './NativeThemeProvider';
import { useColorTheme, readStoredColorTheme } from '@/shared/theme';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke }));
const key = 'auralis:color-theme:v1';
let stored: StoredTheme | null;
let revision = 0;

describe('theme persistence ownership', () => {
  it('serializes rapid choices so a delayed older read cannot overwrite the latest choice', async () => {
    const { result } = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(result.current.persistenceStatus).toBe('idle'));
    let resume!: (theme: StoredTheme | null) => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resume = resolve;
        }),
    );
    act(() => result.current.setColorTheme('frost'));
    await waitFor(() => expect(resume).toBeDefined());
    act(() => result.current.setColorTheme('sandstone'));
    await act(async () => resume(null));
    await waitFor(() => expect(result.current.persistenceStatus).toBe('saved'));
    expect(result.current.colorTheme).toBe('sandstone');
    expect(stored?.value).toBe('sandstone');
    expect(
      invoke.mock.calls
        .filter(([command]) => command === 'set_color_theme_cmd')
        .map(([, args]) => args.value),
    ).toEqual(['frost', 'sandstone']);
  });
  beforeEach(() => {
    localStorage.clear();
    stored = null;
    invoke.mockReset().mockImplementation(async (command: string, args?: { value: string }) => {
      if (command === 'get_color_theme_cmd') return stored;
      if (command === 'import_color_theme_cmd' && stored) return stored;
      stored = { value: args!.value, revision: ++revision };
      return stored;
    });
  });
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('does not overwrite an unreadable legacy value', async () => {
    localStorage.setItem(key, 'frost');
    const read = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('unavailable');
    });
    const write = vi.spyOn(Storage.prototype, 'setItem');
    const { result } = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(result.current.persistenceStatus).toBe('error'));
    expect(result.current.colorTheme).toBe('auralis');
    expect(readStoredColorTheme().status).toBe('unavailable');
    expect(write).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledTimes(1);
    read.mockRestore();
    expect(localStorage.getItem(key)).toBe('frost');
  });

  it('preserves unknown values without importing a default', async () => {
    expect(readStoredColorTheme().status).toBe('missing');
    localStorage.setItem(key, 'future-theme');
    const { result } = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(result.current.persistenceStatus).toBe('error'));
    expect(localStorage.getItem(key)).toBe('future-theme');
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it('uses SQLite without reading legacy data when a native value exists', async () => {
    stored = { value: 'frost', revision: ++revision };
    const read = vi.spyOn(Storage.prototype, 'getItem');
    const { result } = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(result.current.colorTheme).toBe('frost'));
    expect(read).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it('removes legacy data only after acknowledgement and survives WebView clearing', async () => {
    localStorage.setItem(key, 'frost');
    let acknowledge!: (value: StoredTheme) => void;
    invoke
      .mockImplementationOnce(async () => null)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            acknowledge = resolve;
          }),
      );
    const first = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
    expect(localStorage.getItem(key)).toBe('frost');
    stored = { value: 'frost', revision: ++revision };
    await act(async () => acknowledge(stored!));
    expect(localStorage.getItem(key)).toBeNull();
    first.unmount();
    localStorage.clear();
    const next = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(next.result.current.colorTheme).toBe('frost'));
  });

  it('retains legacy data when migration fails and retries on remount', async () => {
    localStorage.setItem(key, 'frost');
    invoke.mockImplementationOnce(async () => null).mockRejectedValueOnce(new Error('disk'));
    const first = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(first.result.current.persistenceStatus).toBe('error'));
    expect(localStorage.getItem(key)).toBe('frost');
    first.unmount();
    const next = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(next.result.current.colorTheme).toBe('frost'));
    expect(localStorage.getItem(key)).toBeNull();
  });

  it('reports failed writes, keeps the session choice and permits explicit retry', async () => {
    const { result } = renderHook(useColorTheme, { wrapper: ThemeProvider });
    await waitFor(() => expect(result.current.persistenceStatus).toBe('idle'));
    invoke.mockImplementationOnce(async () => stored).mockRejectedValueOnce(new Error('disk'));
    act(() => result.current.setColorTheme('frost'));
    expect(result.current.persistenceStatus).toBe('pending');
    await waitFor(() => expect(result.current.persistenceStatus).toBe('error'));
    expect(result.current.colorTheme).toBe('frost');
    act(() => result.current.setColorTheme('frost'));
    await waitFor(() => expect(result.current.persistenceStatus).toBe('saved'));
    expect(stored?.value).toBe('frost');
    expect(localStorage.getItem(key)).toBeNull();
  });

  it('does not apply late hydration over an explicit choice', async () => {
    let hydrate!: (value: StoredTheme) => void;
    invoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          hydrate = resolve;
        }),
    );
    const { result } = renderHook(useColorTheme, { wrapper: ThemeProvider });
    const old = { value: 'sandstone', revision: ++revision };
    act(() => result.current.setColorTheme('frost'));
    await waitFor(() => expect(result.current.persistenceStatus).toBe('saved'));
    await act(async () => hydrate(old));
    expect(result.current.colorTheme).toBe('frost');
    expect(result.current.persistenceStatus).toBe('saved');
  });
});
