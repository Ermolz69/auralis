// @vitest-environment jsdom
import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import {
  listProjects,
  projectRemoved,
  projectUpdated,
  updateProjectPreferences,
  type Project,
} from '@/entities/project';
import { usePinnedProjects } from './usePinnedProjects';

vi.mock('@/entities/project', async (original) => ({
  ...(await original<typeof import('@/entities/project')>()),
  listProjects: vi.fn(),
}));
const project: Project = {
  id: 'pinned',
  title: 'Before',
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};
beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  updateProjectPreferences(project.id, { pinned: true });
  vi.mocked(listProjects).mockResolvedValue([project]);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it('updates and removes pinned projects without a localStorage notification write', async () => {
  const { result } = renderHook(usePinnedProjects);
  await waitFor(() => expect(result.current).toEqual([project]));
  const storage = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new DOMException('blocked', 'SecurityError');
  });
  vi.mocked(listProjects).mockRejectedValue(new Error('refresh unavailable'));
  act(() => projectUpdated({ ...project, title: 'After' }));
  expect(result.current[0].title).toBe('After');
  act(() => projectRemoved(project.id));
  expect(result.current).toEqual([]);
  expect(storage).not.toHaveBeenCalled();
});

it('does not allow an older list response to undo a confirmed rename', async () => {
  const { result } = renderHook(usePinnedProjects);
  await waitFor(() => expect(result.current).toEqual([project]));
  let resolve!: (projects: Project[]) => void;
  vi.mocked(listProjects).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  act(() => updateProjectPreferences(project.id, { pinned: true }));
  const updated = { ...project, title: 'After' };
  vi.mocked(listProjects).mockResolvedValue([updated]);
  act(() => projectUpdated(updated));
  await act(async () => resolve([project]));
  expect(result.current).toEqual([updated]);
});

it('refreshes other pinned projects when deletion races the initial load', async () => {
  let resolve!: (projects: Project[]) => void;
  vi.mocked(listProjects).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const other = { ...project, id: 'other' };
  updateProjectPreferences(other.id, { pinned: true });
  const { result } = renderHook(usePinnedProjects);
  vi.mocked(listProjects).mockResolvedValue([other]);
  act(() => projectRemoved(project.id));
  await waitFor(() => expect(result.current).toEqual([other]));
  await act(async () => resolve([project, other]));
  expect(result.current).toEqual([other]);
});
