// @vitest-environment jsdom
import { cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Project } from '@/shared/api/contracts/project';
import type { LibraryAsset } from '@/shared/api/contracts/library';
import { useSourceOwnership } from './useSourceOwnership';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke }));
afterEach(cleanup);
beforeEach(() => invoke.mockReset());
const project: Project = {
  id: 'project',
  title: 'Story',
  revision: 1,
  status: 'ready_for_processing',
  source: { kind: 'youtubeUrl', url: 'https://www.youtube.com/watch?v=source' },
  metadata: null,
  createdAt: '',
  updatedAt: '',
};
const asset: LibraryAsset = {
  id: 'material',
  ownerProjectId: project.id,
  artifactId: 'source-video',
  name: 'Story',
  kind: 'video',
  format: null,
  text: null,
  parentId: null,
  tags: [],
  openedAt: null,
};

it('shows source controls for the ready downloaded YouTube artifact', async () => {
  invoke.mockResolvedValue([{ id: asset.artifactId, state: 'ready' }]);
  const { result } = renderHook(() => useSourceOwnership(project, asset));
  await waitFor(() => expect(result.current).toBe(true));
  expect(invoke).toHaveBeenCalledWith('list_project_artifacts_cmd', {
    projectId: project.id,
    kind: 'downloadedVideo',
  });
});

it.each(['staged', 'missing', 'different'])(
  'hides project source controls for a %s artifact',
  async (state) => {
    invoke.mockResolvedValue(
      state === 'missing'
        ? []
        : [
            {
              id: state === 'different' ? 'other-video' : asset.artifactId,
              state: state === 'staged' ? 'staged' : 'ready',
            },
          ],
    );
    const { result } = renderHook(() => useSourceOwnership(project, asset));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    expect(result.current).toBe(false);
  },
);

it('does not offer another project subtitle tracks for a foreign material', () => {
  const { result } = renderHook(() =>
    useSourceOwnership(project, { ...asset, ownerProjectId: 'foreign' }),
  );
  expect(result.current).toBe(false);
  expect(invoke).not.toHaveBeenCalled();
});

it('recognizes only the selected managed local source', () => {
  const local: Project = {
    ...project,
    source: { kind: 'managedLocalFile', artifactId: 'source-video', originalFilename: 'story.mp4' },
  };
  const { result, rerender } = renderHook(({ selected }) => useSourceOwnership(local, selected), {
    initialProps: { selected: asset },
  });
  expect(result.current).toBe(true);
  rerender({ selected: { ...asset, artifactId: 'exported-video' } });
  expect(result.current).toBe(false);
  expect(invoke).not.toHaveBeenCalled();
});
