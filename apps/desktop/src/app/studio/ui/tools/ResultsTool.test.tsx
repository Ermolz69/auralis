import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { ResultsTool } from './ResultsTool';
import {
  artifact,
  asset,
  backend,
  deferred,
  LibraryHarness,
  project,
} from '../../../../../tests/studio';
const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => api);
vi.mock('@tauri-apps/api/core', async (original) => ({
  ...(await original<typeof import('@tauri-apps/api/core')>()),
  convertFileSrc: (path: string) => `asset:${path}`,
}));
let data: ReturnType<typeof backend>;
beforeEach(() => {
  vi.resetAllMocks();
  data = backend();
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
});
afterEach(cleanup);
const view = (selected = asset, projectId = project.id) => (
  <LibraryHarness>
    <ResultsTool projectId={projectId} asset={selected} />
  </LibraryHarness>
);
it('offers playback only for ready managed artifacts related through material descendants or project results', async () => {
  data.state.assets = [
    asset,
    { ...asset, id: 'child', parentId: asset.id, artifactId: 'child-audio' },
    { ...asset, id: 'grandchild', parentId: 'child', artifactId: 'grandchild-audio' },
  ];
  data.artifacts = [
    artifact('child-audio'),
    artifact('grandchild-audio'),
    artifact('result-audio'),
    artifact('foreign'),
    artifact('pending', { state: 'pendingFinalize' }),
    artifact('local', { location: { kind: 'localPath', value: 'external.wav' } }),
  ];
  data.overrides.get_media_project_cmd = () => ({
    results: [
      { id: 'result-audio', asset_id: asset.id },
      { id: 'pending', asset_id: asset.id },
      { id: 'local', asset_id: asset.id },
    ],
  });
  render(view());
  await screen.findByText(/^child-audio.wav/);
  expect(screen.getAllByRole('button', { name: 'Прослушать' })).toHaveLength(3);
  expect(screen.queryByText(/foreign.wav|pending.wav|external.wav/)).toBeNull();
  fireEvent.click(screen.getAllByRole('button', { name: 'Прослушать' })[0]);
  await waitFor(() =>
    expect(screen.getByLabelText('Прослушивание результата')).toHaveProperty(
      'src',
      'asset:C:/fixture/child-audio.wav',
    ),
  );
  expect(api.invoke).toHaveBeenCalledWith('resolve_artifact_path_cmd', {
    artifactId: 'child-audio',
  });
});
it('shows all ready project results when no material is selected and keeps non-audio results non-playable', async () => {
  data.artifacts = [
    artifact('audio'),
    artifact('video', { kind: 'finalVideo' }),
    artifact('failed', { state: 'failed' }),
  ];
  render(
    <LibraryHarness>
      <ResultsTool projectId={project.id} />
    </LibraryHarness>,
  );
  await screen.findByText(/video.wav/);
  expect(screen.getAllByRole('button', { name: 'Прослушать' })).toHaveLength(1);
  expect(screen.queryByText(/failed.wav/)).toBeNull();
});
it('does not replace a new material with a late old result response', async () => {
  const old = deferred<ReturnType<typeof artifact>[]>();
  data.overrides.list_project_artifacts_cmd = () => old.promise;
  const rendered = render(view());
  data.overrides.list_project_artifacts_cmd = () => [artifact('new-audio')];
  const next = { ...asset, id: 'new', artifactId: 'new-audio', name: 'New material' };
  data.state.assets.push(next);
  rendered.rerender(view(next));
  await screen.findByText(/new-audio.wav/);
  await act(async () => old.resolve([artifact('old-audio')]));
  expect(screen.queryByText(/old-audio.wav/)).toBeNull();
});
it('ignores a playback resolver that returns after switching project/material', async () => {
  data.state.assets[0].artifactId = 'old-audio';
  data.artifacts = [artifact('old-audio')];
  const pending = deferred<string>();
  data.overrides.resolve_artifact_path_cmd = () => pending.promise;
  const rendered = render(view());
  fireEvent.click(await screen.findByRole('button', { name: 'Прослушать' }));
  rendered.rerender(view({ ...asset, id: 'new' }, 'other-project'));
  await act(async () => pending.resolve('C:/fixture/old.wav'));
  expect(screen.queryByLabelText('Прослушивание результата')).toBeNull();
});
it('admits one resolver for repeated playback and reports missing artifact errors', async () => {
  data.state.assets[0].artifactId = 'audio';
  data.artifacts = [artifact('audio')];
  const pending = deferred<string>();
  data.overrides.resolve_artifact_path_cmd = () => pending.promise;
  render(view());
  const play = await screen.findByRole('button', { name: 'Прослушать' });
  act(() => {
    fireEvent.click(play);
    fireEvent.click(play);
  });
  expect(
    api.invoke.mock.calls.filter(([command]) => command === 'resolve_artifact_path_cmd'),
  ).toHaveLength(1);
  await act(async () => pending.reject(new Error('Artifact missing; refresh results')));
  expect((await screen.findByRole('alert')).textContent).toContain('Artifact missing');
  expect(screen.queryByLabelText('Прослушивание результата')).toBeNull();
});
it('reports refresh publication failures and allows retry without an unhandled rejection', async () => {
  render(view());
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Обновить результаты' })).toHaveProperty(
      'disabled',
      false,
    ),
  );
  data.overrides.get_library_cmd = () => {
    throw new Error('Library offline');
  };
  fireEvent.click(screen.getByRole('button', { name: 'Обновить результаты' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Library offline');
  delete data.overrides.get_library_cmd;
  fireEvent.click(screen.getByRole('button', { name: 'Обновить результаты' }));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
});
it('shows API loading failures and recovers when refreshed', async () => {
  data.overrides.get_media_project_cmd = () => {
    throw new Error('Results unavailable');
  };
  render(view());
  expect((await screen.findByRole('alert')).textContent).toContain('Results unavailable');
  delete data.overrides.get_media_project_cmd;
  fireEvent.click(screen.getByRole('button', { name: 'Обновить результаты' }));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
});
