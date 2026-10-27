import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { emptyLibrary, type LibraryState } from '@/shared/api/contracts/library';
import { LibraryProvider } from './LibraryProvider';
import { useLibrary } from './context';
import { StudioActionsContext, useStudioActions } from './actions';

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke, listen }));
const wrapper = ({ children }: { children: ReactNode }) => (
  <LibraryProvider>{children}</LibraryProvider>
);
const deferred = <T,>() => {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
};
let catalog: LibraryState;
let handlers: Record<string, (event: any) => void>;
let stops: ReturnType<typeof vi.fn>[];
beforeEach(() => {
  catalog = emptyLibrary();
  handlers = {};
  stops = [];
  invoke.mockReset().mockImplementation(async (command: string, args: any) => {
    if (command === 'get_library_cmd') return structuredClone(catalog);
    if (command === 'list_projects_cmd') return [];
    if (command === 'save_library_cmd')
      return (catalog = { ...args.state, revision: args.state.revision + 1 });
    if (command === 'validate_media_project_cmd') return null;
    throw new Error('Unexpected command: ' + command);
  });
  listen.mockReset().mockImplementation(async (event, callback) => {
    handlers[event] = callback;
    const stop = vi.fn();
    stops.push(stop);
    return stop;
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

async function ready() {
  const hook = renderHook(useLibrary, { wrapper });
  await waitFor(() => expect(hook.result.current.loading).toBe(false));
  return hook;
}

it('requires a provider and exposes harmless default studio actions', () => {
  expect(() => renderHook(useLibrary)).toThrow('LibraryProvider is required');
  const { result } = renderHook(useStudioActions);
  expect(() => {
    result.current.select({ projectId: null, assetId: null });
    result.current.run('properties');
    result.current.context({} as React.MouseEvent, { projectId: null, assetId: null });
    result.current.drop('asset', 'project');
  }).not.toThrow();
  const run = vi.fn();
  const value = { ...result.current, run };
  const provided = renderHook(useStudioActions, {
    wrapper: ({ children }) => (
      <StudioActionsContext.Provider value={value}>{children}</StudioActionsContext.Provider>
    ),
  });
  provided.result.current.run('new-project');
  expect(run).toHaveBeenCalledWith('new-project');
});

it('serializes refresh after accepted writes and never overwrites with an earlier read', async () => {
  const { result } = await ready();
  const save = deferred<LibraryState>();
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'save_library_cmd') return save.promise;
    return original(command, args);
  });
  let writing!: Promise<void>;
  let refreshing!: Promise<void>;
  act(() => {
    writing = result.current.mutate((draft) => {
      draft.browser.search = 'Accepted';
    });
    refreshing = result.current.refresh();
  });
  await waitFor(() => expect(invoke).toHaveBeenCalledWith('save_library_cmd', expect.anything()));
  expect(invoke.mock.calls.filter(([command]) => command === 'get_library_cmd')).toHaveLength(1);
  catalog = { ...emptyLibrary(), revision: 1, browser: { ...catalog.browser, search: 'Accepted' } };
  await act(async () => {
    save.resolve(catalog);
    await writing;
    await refreshing;
  });
  expect(result.current.state.browser.search).toBe('Accepted');
  expect(result.current.state.revision).toBe(1);
});

it('rebases a stale revision once while preserving concurrent catalog changes', async () => {
  const { result } = await ready();
  catalog = { ...emptyLibrary(), revision: 7, standaloneOwners: ['concurrent-owner'] };
  const original = invoke.getMockImplementation()!;
  let attempts = 0;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'save_library_cmd' && attempts++ === 0)
      throw { code: 'CONFLICT', message: 'Stale revision' };
    return original(command, args);
  });
  await act(async () => {
    await result.current.mutate((draft) => {
      draft.browser.search = 'Mine';
    });
  });
  const writes = invoke.mock.calls.filter(([command]) => command === 'save_library_cmd');
  expect(writes.map(([, args]) => args.state.revision)).toEqual([0, 7]);
  expect(result.current.state).toMatchObject({
    revision: 8,
    standaloneOwners: ['concurrent-owner'],
    browser: { search: 'Mine' },
  });
});

it('bounds repeated conflicts and leaves the latest accepted state available for retry', async () => {
  const { result } = await ready();
  catalog = { ...emptyLibrary(), revision: 9, standaloneOwners: ['accepted'] };
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'save_library_cmd') throw { code: 'CONFLICT', message: 'Reload and retry' };
    return original(command, args);
  });
  await act(async () => {
    await expect(
      result.current.mutate((draft) => {
        draft.hiddenAssets.push('not-saved');
      }),
    ).rejects.toMatchObject({ code: 'CONFLICT' });
  });
  expect(invoke.mock.calls.filter(([command]) => command === 'save_library_cmd')).toHaveLength(2);
  expect(result.current.state).toMatchObject({
    revision: 9,
    hiddenAssets: [],
    standaloneOwners: ['accepted'],
  });
  expect(result.current.error).toBe('Reload and retry');
  invoke.mockImplementation(original);
  await act(async () => {
    await result.current.mutate((draft) => {
      draft.hiddenAssets.push('retry');
    });
  });
  expect(result.current.state.hiddenAssets).toEqual(['retry']);
  expect(result.current.error).toBeNull();
});

it('preserves accepted state on save failure and lets the next queued operation recover', async () => {
  const { result } = await ready();
  const previous = result.current.state;
  invoke.mockRejectedValueOnce({ code: 'IO', message: 'Disk unavailable. Retry saving.' });
  await act(async () => {
    await expect(
      result.current.mutate((draft) => {
        draft.standaloneOwners.push('lost');
      }),
    ).rejects.toMatchObject({ code: 'IO' });
  });
  expect(result.current.state).toBe(previous);
  expect(result.current.error).toBe('Disk unavailable. Retry saving.');
  await act(async () => {
    await result.current.refresh();
  });
  expect(result.current.error).toBeNull();
});

it('retains both catalog and projects if either refresh response fails', async () => {
  const { result } = await ready();
  const previous = result.current.state;
  const original = invoke.getMockImplementation()!;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'list_projects_cmd') throw new Error('Projects unavailable');
    return original(command, args);
  });
  await act(async () => {
    await expect(result.current.refresh()).rejects.toThrow('Projects unavailable');
  });
  expect(result.current.state).toBe(previous);
  expect(result.current.loading).toBe(false);
  expect(result.current.error).toBe('Projects unavailable');
});

it('debounces project and terminal job events and removes listeners and pending refresh on unmount', async () => {
  const hook = await ready();
  vi.useFakeTimers();
  invoke.mockClear();
  act(() => {
    handlers['job-event']({ payload: { kind: 'progress' } });
  });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(300);
  });
  expect(invoke).not.toHaveBeenCalled();
  act(() => {
    handlers['project-updated']({});
    handlers['job-event']({ payload: { kind: 'completed' } });
    handlers['job-event']({ payload: { kind: 'failed' } });
    handlers['job-event']({ payload: { kind: 'cancelled' } });
  });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(250);
  });
  expect(invoke.mock.calls.filter(([command]) => command === 'get_library_cmd')).toHaveLength(1);
  act(() => handlers['project-updated']({}));
  hook.unmount();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(300);
  });
  expect(invoke.mock.calls.filter(([command]) => command === 'get_library_cmd')).toHaveLength(1);
  stops.forEach((stop) => expect(stop).toHaveBeenCalledOnce());
});

it('releases each late listener exactly once after unmount', async () => {
  const first = deferred<() => void>();
  const second = deferred<() => void>();
  listen.mockReset().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
  const hook = await ready();
  hook.unmount();
  const stopFirst = vi.fn();
  const stopSecond = vi.fn();
  await act(async () => {
    first.resolve(stopFirst);
    second.resolve(stopSecond);
  });
  expect(stopFirst).toHaveBeenCalledOnce();
  expect(stopSecond).toHaveBeenCalledOnce();
});

it('releases an admitted listener even if another subscription fails', async () => {
  const stop = vi.fn();
  listen
    .mockReset()
    .mockResolvedValueOnce(stop)
    .mockRejectedValueOnce(new Error('Events unavailable'));
  const hook = await ready();
  hook.unmount();
  expect(stop).toHaveBeenCalledOnce();
});

it('exposes validation recovery and retries it only after the catalog refresh', async () => {
  const { result } = await ready();
  const original = invoke.getMockImplementation()!;
  let invalid = true;
  invoke.mockImplementation(async (command, args) => {
    if (command === 'validate_media_project_cmd' && invalid)
      throw {
        code: 'RECOVERY_REQUIRED',
        message: 'Runtime unavailable. Restore the project runtime and refresh.',
      };
    return original(command, args);
  });
  await act(async () => {
    await result.current.openProject('project');
  });
  expect(result.current.error).toContain('Restore the project runtime and refresh');
  await act(async () => {
    await result.current.refresh();
  });
  expect(result.current.error).toContain('Restore the project runtime and refresh');
  invalid = false;
  invoke.mockClear();
  await act(async () => {
    await result.current.refresh();
  });
  expect(result.current.error).toBeNull();
  const commands = invoke.mock.calls.map(([command]) => command);
  expect(commands.indexOf('get_library_cmd')).toBeLessThan(
    commands.indexOf('validate_media_project_cmd'),
  );
  invoke.mockClear();
  await act(async () => {
    await result.current.refresh();
  });
  expect(invoke.mock.calls.some(([command]) => command === 'validate_media_project_cmd')).toBe(
    false,
  );
});

it('opens standalone media against its owner and uses the project context only when selected', async () => {
  const media = {
    id: 'media',
    ownerProjectId: 'owner',
    name: 'Media',
    kind: 'audio' as const,
    text: null,
    artifactId: 'file',
    format: 'wav',
    parentId: null,
    tags: [],
    openedAt: null,
  };
  catalog.assets.push(media);
  const { result } = await ready();
  await act(async () => {
    await result.current.openAsset(media);
  });
  expect(invoke).toHaveBeenCalledWith('validate_media_project_cmd', { projectId: 'owner' });
  expect(result.current.state.assets[0].openedAt).toBeTruthy();
  expect(result.current.state.workspaces[0].tabs[0].tool).toBe('results');
  await act(async () => {
    await result.current.mutate((draft) => {
      draft.browser.scope = 'project';
      draft.browser.projectId = 'project';
    });
  });
  await act(async () => {
    await result.current.openAsset(media, 'voices');
  });
  expect(invoke).toHaveBeenLastCalledWith('save_library_cmd', expect.anything());
  expect(invoke).toHaveBeenCalledWith('validate_media_project_cmd', { projectId: 'project' });
  expect(result.current.state.workspaces.find((w) => w.projectId === 'project')?.tabs[0].tool).toBe(
    'voices',
  );
});
