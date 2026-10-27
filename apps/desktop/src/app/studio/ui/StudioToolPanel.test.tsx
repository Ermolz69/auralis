import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { StudioToolPanel } from './StudioToolPanel';
import { asset, backend, deferred, LibraryHarness, project } from '../../../../tests/studio';
const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => api);
let data: ReturnType<typeof backend>;
const tab = { id: 'tab', tool: 'text' as const, assetId: asset.id, draft: '{}' };
const workspace = {
  id: 'workspace',
  projectId: project.id,
  assetId: null,
  activeTab: tab.id,
  tabs: [tab],
};
const view = (selected = tab) => (
  <LibraryHarness>
    <StudioToolPanel tab={selected} workspace={workspace} />
  </LibraryHarness>
);
beforeEach(() => {
  vi.resetAllMocks();
  data = backend();
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
});
afterEach(cleanup);
it('waits for verified project/material context before rendering processing controls', async () => {
  const pending = deferred<unknown>();
  data.overrides.get_tool_context_cmd = () => pending.promise;
  render(view());
  expect(screen.queryByRole('textbox')).toBeNull();
  await act(async () =>
    pending.resolve({
      projectId: project.id,
      assetId: asset.id,
      artifactId: null,
      version: null,
      trackId: null,
    }),
  );
  expect(await screen.findByRole('textbox')).toHaveProperty('value', asset.text);
});
it.each(['project', 'material', 'missing-artifact'])(
  'rejects a foreign or partial %s context',
  async (invalid) => {
    if (invalid === 'missing-artifact') data.state.assets[0].artifactId = 'required-artifact';
    data.overrides.get_tool_context_cmd = () => ({
      projectId: invalid === 'project' ? 'foreign' : project.id,
      assetId: invalid === 'material' ? 'foreign' : asset.id,
      artifactId: null,
      version: null,
      trackId: null,
    });
    render(view());
    expect(await screen.findByRole('alert')).toBeTruthy();
    expect(screen.queryByRole('textbox')).toBeNull();
    expect(api.invoke.mock.calls.some(([command]) => command === 'get_speech_catalog_cmd')).toBe(
      false,
    );
  },
);
it('rejects a project response with a different owner identity', async () => {
  data.overrides.get_project_cmd = () => ({ ...project, id: 'foreign' });
  render(view());
  expect(await screen.findByRole('alert')).toBeTruthy();
  expect(screen.queryByRole('textbox')).toBeNull();
  delete data.overrides.get_project_cmd;
  fireEvent.click(screen.getByRole('button', { name: 'Повторить проверку материала' }));
  expect(await screen.findByRole('textbox')).toHaveProperty('value', asset.text);
});
it('does not expose processing after material verification fails', async () => {
  data.overrides.get_tool_context_cmd = () => {
    throw new Error('Material has not been published');
  };
  render(view());
  expect((await screen.findByRole('alert')).textContent).toContain(
    'Material has not been published',
  );
  expect(screen.queryByRole('textbox')).toBeNull();
});
it('uses a project-owned mapped artifact rather than the original library artifact', async () => {
  data.state.assets[0] = { ...asset, text: null, artifactId: 'original-artifact' };
  data.overrides.get_tool_context_cmd = () => ({
    projectId: project.id,
    assetId: asset.id,
    artifactId: 'mapped-artifact',
    version: 'version',
    trackId: null,
  });
  data.overrides.read_library_text_cmd = () => 'Mapped text';
  render(view());
  expect(await screen.findByRole('textbox')).toHaveProperty('value', 'Mapped text');
  expect(api.invoke).toHaveBeenCalledWith('resolve_artifact_path_cmd', {
    artifactId: 'mapped-artifact',
  });
  expect(api.invoke).not.toHaveBeenCalledWith('resolve_artifact_path_cmd', {
    artifactId: 'original-artifact',
  });
});
it('does not render a late old project/material after switching tabs', async () => {
  data.state.assets.push({ ...asset, id: 'new-material', text: 'New material text' });
  const pending = deferred<unknown>();
  data.overrides.get_tool_context_cmd = () => pending.promise;
  const rendered = render(view());
  delete data.overrides.get_tool_context_cmd;
  rendered.rerender(view({ ...tab, assetId: 'new-material' }));
  expect(await screen.findByRole('textbox')).toHaveProperty('value', 'New material text');
  await act(async () =>
    pending.resolve({ projectId: project.id, assetId: asset.id, artifactId: null }),
  );
  expect(screen.getByRole('textbox')).toHaveProperty('value', 'New material text');
});
it('reports project API failures', async () => {
  data.overrides.get_project_cmd = () => {
    throw new Error('Project offline');
  };
  render(view());
  expect((await screen.findByRole('alert')).textContent).toContain('Project offline');
});
