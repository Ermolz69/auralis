// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@/shared/api/tauri';
import { ProjectProvider } from './ProjectProvider';
import { useProjectContext } from './useProjectContext';
import type { Project } from './types';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke: vi.fn() }));

const project: Project = {
  id: 'p1',
  title: 'Selected',
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};
let eventCallback: (event: { payload: { projectId: string } }) => Promise<void>;
const missing = { code: 'NOT_FOUND', message: 'Project no longer exists' };
const emit = () => eventCallback({ payload: { projectId: 'p1' } });

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(listen).mockImplementation((_name, callback) => {
    eventCallback = callback as typeof eventCallback;
    return Promise.resolve(vi.fn());
  });
  vi.spyOn(console, 'warn').mockImplementation(() => {});
  vi.spyOn(console, 'error').mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function setup() {
  return renderHook(useProjectContext, { wrapper: ProjectProvider });
}

describe('atomic project selection', () => {
  it('closes a missing project, clears both derived values and invalidates operations', async () => {
    const { result } = setup();
    expect(result.current.selection).toEqual({ status: 'closed' });
    await act(async () => result.current.setProject(project));
    const token = result.current.captureToken();
    vi.mocked(invoke).mockRejectedValueOnce(missing);
    await act(async () => {
      await emit();
      expect(result.current.validateToken(token)).toBe(false);
      expect(result.current.captureToken().projectId).toBeNull();
    });
    expect(result.current.selection).toEqual({ status: 'closed' });
    expect(result.current.projectId).toBeNull();
    expect(result.current.project).toBeNull();
  });

  it('does not close a new selection when an old NOT_FOUND response arrives', async () => {
    const { result } = setup();
    await act(async () => result.current.setProject(project));
    let reject!: (reason: unknown) => void;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise((_resolve, fail) => {
        reject = fail;
      }),
    );
    let pending!: Promise<void>;
    act(() => {
      pending = emit();
    });
    const next = { ...project, id: 'p2' };
    await act(async () => result.current.setProject(next));
    const token = result.current.captureToken();
    await act(async () => {
      reject(missing);
      await pending;
    });
    expect(result.current.selection).toEqual({ status: 'open', project: next });
    expect(result.current.validateToken(token)).toBe(true);
  });

  it('does not reopen a missing project when an older successful response arrives', async () => {
    const { result } = setup();
    await act(async () => result.current.setProject(project));
    let resolve!: (value: Project) => void;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise((done) => {
        resolve = done;
      }),
    );
    let pending!: Promise<void>;
    act(() => {
      pending = emit();
    });
    vi.mocked(invoke).mockRejectedValueOnce(missing);
    await act(emit);
    await act(async () => {
      resolve(project);
      await pending;
    });
    expect(result.current.selection.status).toBe('closed');
  });

  it('ignores an older missing response after a newer successful sync', async () => {
    const { result } = setup();
    await act(async () => result.current.setProject(project));
    let reject!: (reason: unknown) => void;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise((_resolve, fail) => {
        reject = fail;
      }),
    );
    let pending!: Promise<void>;
    act(() => {
      pending = emit();
    });
    vi.mocked(invoke).mockResolvedValueOnce({ ...project, title: 'Current' });
    await act(emit);
    await act(async () => {
      reject(missing);
      await pending;
    });
    expect(result.current.project?.title).toBe('Current');
  });

  it('keeps selection and operations on a transient backend failure', async () => {
    const { result } = setup();
    await act(async () => result.current.setProject(project));
    const token = result.current.captureToken();
    vi.mocked(invoke).mockRejectedValueOnce({ code: 'BUSY', message: 'Try again' });
    await act(emit);
    expect(result.current.project).toEqual(project);
    expect(result.current.validateToken(token)).toBe(true);
  });

  it('invalidates immediately on close, including reopening the same project before rerender', async () => {
    const { result } = setup();
    await act(async () => result.current.setProject(project));
    const token = result.current.captureToken();
    act(() => {
      result.current.setProject(null);
      expect(result.current.validateToken(token)).toBe(false);
      result.current.setProject(project);
      expect(result.current.validateToken(token)).toBe(false);
    });
    expect(result.current.projectId).toBe(project.id);
    expect(result.current.project).toEqual(project);
  });
});
