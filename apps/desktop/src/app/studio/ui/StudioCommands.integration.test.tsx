import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useLibrary, useStudioActions, type StudioAction } from '@/entities/library';
import { StudioCommands, StudioMenuBar } from './StudioCommands';
import {
  backend,
  asset,
  project,
  deferred,
  dialogSupport,
  LibraryHarness,
  snapshot,
} from '../../../../tests/studio';

const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn(), open: vi.fn() }));
vi.mock('@/shared/api/tauri', () => ({ invoke: api.invoke, listen: api.listen }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: api.open }));
let data: ReturnType<typeof backend>;
const actions: StudioAction[] = [
  'rename-project',
  'rename-asset',
  'delete-project',
  'remove-asset',
  'properties',
  'attach',
  'import',
  'open-project',
  'open-asset',
  'tool:source',
  'tool:translate',
  'tool:speech',
  'tool:voices',
  'tool:results',
  'new-project',
  'new-text',
  'youtube',
  'settings',
  'exit',
];
function Controls() {
  const commands = useStudioActions();
  const library = useLibrary();
  return (
    <>
      <StudioMenuBar />
      {actions.map((action) => (
        <button key={action} onClick={() => commands.run(action)}>
          Run {action}
        </button>
      ))}
      <button onClick={() => commands.select({ projectId: project.id, assetId: asset.id })}>
        Select material
      </button>
      <button onClick={() => commands.select({ projectId: null, assetId: null })}>
        Select background
      </button>
      <button
        onClick={() =>
          commands.select({
            projectId: 'foreign',
            assetId: 'foreign',
            workspaceId: 'old-workspace',
          })
        }
      >
        Select stale workspace
      </button>
      <button
        onContextMenu={(event) =>
          commands.context(event, { projectId: project.id, assetId: asset.id })
        }
        onKeyDown={(event) => commands.context(event, { projectId: project.id, assetId: asset.id })}
      >
        Context target
      </button>
      <button onClick={() => commands.drop(asset.id, project.id)}>Drop material</button>
      <span aria-label="Current view">{library.view}</span>
    </>
  );
}
beforeEach(() => {
  vi.resetAllMocks();
  dialogSupport();
  data = backend();
  data.state.browser = {
    ...data.state.browser,
    scope: 'project',
    projectId: project.id,
    selectedId: asset.id,
  };
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
  api.open.mockResolvedValue(null);
});
afterEach(cleanup);
async function setup() {
  render(
    <LibraryHarness>
      <StudioCommands>
        <Controls />
      </StudioCommands>
    </LibraryHarness>,
  );
  await waitFor(() =>
    expect(screen.getByLabelText('Library snapshot').getAttribute('data-loaded')).toBe('true'),
  );
}
const run = (action: StudioAction) =>
  fireEvent.click(screen.getByRole('button', { name: `Run ${action}` }));
const saved = () => snapshot(screen.getByLabelText('Library snapshot'));
const confirm = () => fireEvent.click(screen.getByRole('button', { name: 'Подтвердить' }));
it('shows bounded copy progress and ignores a late poll after publication', async () => {
  const attaching = deferred<unknown>();
  const poll = deferred<unknown>();
  let count = 0;
  data.overrides.attach_library_asset_cmd = () => attaching.promise;
  data.overrides.get_media_operation_cmd = ({ operationId }) =>
    ++count === 1
      ? poll.promise
      : {
          id: operationId,
          projectId: project.id,
          assetId: asset.id,
          status: 'ready',
          completedBytes: 100,
          totalBytes: 100,
          error: null,
        };
  await setup();
  fireEvent.click(screen.getByRole('button', { name: 'Drop material' }));
  await waitFor(() => expect(count).toBeGreaterThanOrEqual(1));
  const operationId = api.invoke.mock.calls.find(
    ([command]) => command === 'attach_library_asset_cmd',
  )![1].operationId;
  await act(async () => attaching.resolve(data.state));
  await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
  await act(async () =>
    poll.resolve({
      id: operationId,
      projectId: project.id,
      status: 'copying',
      completedBytes: 60,
      totalBytes: 100,
    }),
  );
  expect(screen.queryByRole('status')).toBeNull();
});
it('retains cancellation errors while showing copy byte progress', async () => {
  const pending = deferred<unknown>();
  data.overrides.attach_library_asset_cmd = () => pending.promise;
  data.overrides.get_media_operation_cmd = ({ operationId }) => ({
    id: operationId,
    projectId: project.id,
    assetId: asset.id,
    status: 'copying',
    completedBytes: 60,
    totalBytes: 100,
    error: null,
  });
  data.overrides.cancel_media_operation_cmd = () => {
    throw new Error('Cancellation refused; operation is still active');
  };
  await setup();
  fireEvent.click(screen.getByRole('button', { name: 'Drop material' }));
  await waitFor(() => expect(screen.getByRole('status').textContent).toContain('60%'));
  fireEvent.click(screen.getByRole('button', { name: 'Отменить' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Cancellation refused');
  await act(async () => pending.reject(new Error('Copy failed')));
  await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
});
it('does not declare a missing operation successful without a published project link', async () => {
  data.overrides.attach_library_asset_cmd = () => data.state;
  data.overrides.get_media_operation_cmd = () => null;
  await setup();
  run('attach');
  fireEvent.change(screen.getByRole('combobox', { name: 'Проект назначения' }), {
    target: { value: project.id },
  });
  confirm();
  expect((await screen.findByRole('alert')).textContent).toContain('публикацию материала');
  expect(screen.getByRole('dialog')).toBeTruthy();
});

it('navigates the real file menu with keyboard, enters imports, cancels and restores focus', async () => {
  await setup();
  const file = screen.getByRole('menuitem', { name: 'Файл' });
  file.focus();
  fireEvent.keyDown(file, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Проект' }));
  fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(file);
  fireEvent.keyDown(file, { key: 'ArrowDown' });
  const menu = screen.getByRole('menu');
  expect(document.activeElement).toBe(
    within(menu).getByRole('menuitem', { name: 'Новый проект…' }),
  );
  fireEvent.click(within(menu).getByRole('menuitem', { name: 'Импорт' }));
  expect(within(screen.getByRole('menu')).getByRole('menuitem', { name: 'Файлы…' })).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('menu'), { key: 'ArrowLeft' });
  fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
  expect(screen.queryByRole('menu')).toBeNull();
  expect(document.activeElement).toBe(file);
});
it.each(['ContextMenu', 'F10'])(
  'uses %s context and ignores unrelated keyboard input',
  async (key) => {
    await setup();
    const target = screen.getByRole('button', { name: 'Context target' });
    target.focus();
    fireEvent.keyDown(target, { key: 'a' });
    expect(screen.queryByRole('menu')).toBeNull();
    fireEvent.keyDown(target, { key, shiftKey: key === 'F10' });
    fireEvent.click(within(screen.getByRole('menu')).getByRole('menuitem', { name: 'Свойства…' }));
    expect(screen.getByRole('dialog', { name: 'Свойства материала' }).textContent).toContain(
      asset.name,
    );
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
  },
);
it('renames only the selected project at its accepted revision and keeps the dialog on failure', async () => {
  let fail = true;
  data.overrides.rename_project_cmd = ({ title }) => {
    if (fail) throw new Error('Revision changed; refresh and retry');
    return { ...project, title };
  };
  await setup();
  run('rename-project');
  expect(screen.getByRole('textbox', { name: 'Новое название' })).toHaveProperty(
    'value',
    project.title,
  );
  fireEvent.change(screen.getByRole('textbox'), { target: { value: '  Renamed project  ' } });
  confirm();
  expect((await screen.findByRole('alert')).textContent).toContain('Revision changed');
  expect(screen.getByRole('dialog')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
  expect(screen.queryByRole('alert')).toBeNull();
  fail = false;
  confirm();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(api.invoke).toHaveBeenCalledWith('rename_project_cmd', {
    projectId: project.id,
    expectedRevision: project.revision,
    title: 'Renamed project',
  });
});
it('admits a repeated form submit once and refuses dismissal while a rename is pending', async () => {
  const pending = deferred<typeof project>();
  data.overrides.rename_project_cmd = () => pending.promise;
  await setup();
  run('rename-project');
  const dialog = screen.getByRole('dialog');
  await act(async () => {
    fireEvent.submit(dialog.querySelector('form')!);
    fireEvent.submit(dialog.querySelector('form')!);
  });
  expect(
    api.invoke.mock.calls.filter(([command]) => command === 'rename_project_cmd'),
  ).toHaveLength(1);
  expect(screen.getByRole('button', { name: 'Выполняем…' })).toHaveProperty('disabled', true);
  fireEvent.keyDown(dialog, { key: 'Escape' });
  expect(screen.getByRole('dialog')).toBeTruthy();
  await act(async () => pending.resolve(project));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});
it('cancels rename without persistence and trims a material rename', async () => {
  await setup();
  run('rename-asset');
  fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Unsaved' } });
  fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
  expect(saved().assets[0].name).toBe(asset.name);
  run('rename-asset');
  fireEvent.change(screen.getByRole('textbox'), { target: { value: '  Material renamed  ' } });
  confirm();
  await waitFor(() => expect(saved().assets[0].name).toBe('Material renamed'));
});
it('rejects a whitespace-only name without persisting a blank material', async () => {
  await setup();
  run('rename-asset');
  fireEvent.change(screen.getByRole('textbox'), { target: { value: '   ' } });
  confirm();
  expect((await screen.findByRole('alert')).textContent).toContain('непустое название');
  expect(saved().assets[0].name).toBe(asset.name);
  expect(api.invoke.mock.calls.some(([command]) => command === 'save_library_cmd')).toBe(false);
});
it('imports a picked text through the ordinary dialog into an independent library material', async () => {
  data.state.browser.scope = 'all';
  data.state.browser.projectId = null;
  api.open.mockResolvedValue('C:/fixture/import.txt');
  data.overrides.read_library_text_cmd = () => 'Imported text';
  await setup();
  run('import');
  fireEvent.click(screen.getByRole('button', { name: 'Выбрать файлы…' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(saved().assets.find((item) => item.name === 'import.txt')).toMatchObject({
    text: 'Imported text',
    kind: 'text',
  });
  expect(api.invoke).toHaveBeenCalledWith('read_library_text_cmd', {
    path: 'C:/fixture/import.txt',
  });
});
it('removes a project from its links and workspaces without deleting stored files', async () => {
  data.state.links = [
    { projectId: project.id, assetId: asset.id, folderId: null },
    { projectId: 'other', assetId: asset.id, folderId: null },
  ];
  data.state.workspaces = [
    { id: 'workspace', projectId: project.id, assetId: null, tabs: [], activeTab: null },
  ];
  await setup();
  run('delete-project');
  confirm();
  await waitFor(() => expect(saved().removedProjects).toEqual([project.id]));
  expect(saved().links.map((link) => link.projectId)).toEqual(['other']);
  expect(saved().workspaces).toEqual([]);
  expect(saved().assets).toHaveLength(1);
  expect(api.invoke.mock.calls.some(([command]) => command === 'delete_project_cmd')).toBe(false);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Проект' }));
  expect(
    screen.getByRole('menuitem', { name: 'Переименовать…' }).getAttribute('aria-disabled'),
  ).toBe('true');
});
it.each([true, false])(
  'removes a material in project scope=%s without discarding another project link',
  async (scoped) => {
    data.state.links = [
      { projectId: project.id, assetId: asset.id, folderId: null },
      { projectId: 'other', assetId: asset.id, folderId: null },
    ];
    if (!scoped) data.state.browser = { ...data.state.browser, scope: 'all', projectId: null };
    await setup();
    run('remove-asset');
    confirm();
    await waitFor(() => expect(saved().links).toHaveLength(scoped ? 1 : 0));
    expect(saved().hiddenAssets).toEqual(scoped ? [] : [asset.id]);
    expect(saved().assets).toHaveLength(1);
  },
);
it('opens a material and project using current context rather than an old workspace selection', async () => {
  await setup();
  fireEvent.click(screen.getByRole('button', { name: 'Select stale workspace' }));
  run('open-asset');
  await waitFor(() => expect(saved().workspaces[0]?.tabs[0]?.assetId).toBe(asset.id));
  fireEvent.click(screen.getByRole('button', { name: 'Select material' }));
  run('open-project');
  await waitFor(() => expect(screen.getByLabelText('Current view').textContent).toBe('workspace'));
  expect(api.invoke).toHaveBeenCalledWith('validate_media_project_cmd', { projectId: project.id });
});
it('filters the tool picker by project, hidden state and material type, then opens one selection', async () => {
  data.state.assets = [
    asset,
    { ...asset, id: 'subtitle', name: 'Linked subtitles', kind: 'subtitle' },
    { ...asset, id: 'hidden', name: 'Hidden subtitles', kind: 'subtitle' },
    { ...asset, id: 'foreign', name: 'Foreign subtitles', kind: 'subtitle' },
  ];
  data.state.hiddenAssets = ['hidden'];
  data.state.links = ['subtitle', 'hidden'].map((assetId) => ({
    projectId: project.id,
    assetId,
    folderId: null,
  }));
  data.state.browser.selectedId = null;
  await setup();
  run('tool:translate');
  const picker = screen.getByRole('dialog', { name: 'Выберите материал' });
  expect(within(picker).queryByRole('button', { name: 'Foreign subtitles' })).toBeNull();
  expect(within(picker).queryByRole('button', { name: asset.name })).toBeNull();
  await waitFor(() =>
    expect(within(picker).getByRole('button', { name: 'Linked subtitles' })).toHaveProperty(
      'disabled',
      false,
    ),
  );
  fireEvent.click(within(picker).getByRole('button', { name: 'Linked subtitles' }));
  await waitFor(() => expect(saved().workspaces[0]?.tabs[0]?.assetId).toBe('subtitle'));
  expect(saved().workspaces[0].tabs[0].tool).toBe('translate');
});
it('does not admit an incompatible tool and reuses project voices/results tabs', async () => {
  await setup();
  run('tool:source');
  await act(async () => {});
  expect(saved().workspaces).toEqual([]);
  run('tool:voices');
  await waitFor(() => expect(saved().workspaces[0]?.tabs[0]?.tool).toBe('voices'));
  run('tool:voices');
  await waitFor(() => expect(saved().workspaces[0].tabs).toHaveLength(1));
  run('tool:results');
  await waitFor(() => expect(saved().workspaces[0].tabs).toHaveLength(2));
});
it('cancels native import without changing assets and exposes native picking failures', async () => {
  await setup();
  run('import');
  fireEvent.click(screen.getByRole('button', { name: 'Выбрать файлы…' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(saved().assets).toEqual([asset]);
  api.open.mockRejectedValueOnce(new Error('Picker unavailable'));
  run('import');
  fireEvent.click(screen.getByRole('button', { name: 'Выбрать файлы…' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Picker unavailable');
  expect(screen.getByRole('dialog')).toBeTruthy();
});
it('creates each ordinary dialog in the selected destination and opens/closes settings', async () => {
  await setup();
  for (const action of ['new-project', 'new-text', 'youtube'] as const) {
    run(action);
    expect(screen.getByRole('dialog')).toBeTruthy();
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
  }
  run('settings');
  expect(screen.getByRole('dialog', { name: 'Настройки' })).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть настройки' }));
  expect(screen.queryByRole('dialog')).toBeNull();
  act(() => window.dispatchEvent(new Event('auralis-open-settings')));
  expect(screen.getByRole('dialog')).toBeTruthy();
});
it('shows copy progress, prevents duplicate drops and cancels the admitted operation', async () => {
  const pending = deferred<unknown>();
  data.overrides.attach_library_asset_cmd = () => pending.promise;
  data.overrides.cancel_media_operation_cmd = () => null;
  data.overrides.get_media_operation_cmd = ({ operationId }) => ({
    id: operationId,
    projectId: project.id,
    assetId: asset.id,
    status: 'ready',
    totalBytes: 100,
    completedBytes: 100,
    error: null,
  });
  await setup();
  fireEvent.click(screen.getByRole('button', { name: 'Drop material' }));
  fireEvent.click(screen.getByRole('button', { name: 'Drop material' }));
  expect(
    api.invoke.mock.calls.filter(([command]) => command === 'attach_library_asset_cmd'),
  ).toHaveLength(1);
  expect(screen.getByRole('status').textContent).toContain('Добавление в проект');
  fireEvent.click(screen.getByRole('button', { name: 'Отменить' }));
  expect(api.invoke).toHaveBeenCalledWith('cancel_media_operation_cmd', {
    operationId: expect.any(String),
  });
  await act(async () => pending.resolve(data.state));
  await waitFor(() => expect(screen.queryByRole('status')).toBeNull());
});
it.each(['failed', 'cancelled'])('keeps a %s unpublished copy actionable', async (status) => {
  data.overrides.attach_library_asset_cmd = () => data.state;
  data.overrides.get_media_operation_cmd = ({ operationId }) => ({
    id: operationId,
    projectId: project.id,
    assetId: asset.id,
    status,
    totalBytes: 0,
    completedBytes: 0,
    error: null,
  });
  await setup();
  run('attach');
  fireEvent.change(screen.getByRole('combobox', { name: 'Проект назначения' }), {
    target: { value: project.id },
  });
  confirm();
  expect((await screen.findByRole('alert')).textContent).toMatch(/не|отмен|failed|cancelled/i);
  expect(screen.getByRole('dialog')).toBeTruthy();
  expect(saved().links).toEqual([]);
});
