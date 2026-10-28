import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { LibraryContext, StudioActionsContext, type LibraryContextValue } from '@/entities/library';
import { emptyLibrary, type LibraryAsset, type LibraryState } from '@/shared/api/contracts/library';
import { LibraryCreateDialog } from './LibraryCreateDialog';
import { AssetInspector } from './AssetInspector';
import { AssetVersions } from './AssetVersions';
import { LibraryProjectActions } from './LibraryProjectActions';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@/shared/api/tauri', () => ({ invoke, listen: vi.fn() }));
const asset = (id = 'asset', extra: Partial<LibraryAsset> = {}): LibraryAsset => ({
  id,
  name: id,
  ownerProjectId: 'owner',
  kind: 'text',
  text: 'Accepted words',
  artifactId: null,
  format: 'txt',
  parentId: null,
  tags: ['one'],
  openedAt: null,
  ...extra,
});
let context: LibraryContextValue;
const drop = vi.fn();
beforeEach(() => {
  const state = emptyLibrary();
  state.assets = [asset()];
  context = {
    state,
    projects: [{ id: 'project', title: 'Project', revision: 7 }] as LibraryContextValue['projects'],
    loading: false,
    error: null,
    view: 'library',
    workspaceId: null,
    setView: vi.fn(),
    activateWorkspace: vi.fn(),
    mutate: vi.fn(async (change: (draft: LibraryState) => void) => {
      const draft = structuredClone(state);
      change(draft);
      Object.assign(state, draft);
    }),
    refresh: vi.fn().mockResolvedValue(undefined),
    openAsset: vi.fn().mockResolvedValue(undefined),
    openProject: vi.fn().mockResolvedValue(undefined),
    openTool: vi.fn().mockResolvedValue(undefined),
  };
  drop.mockReset();
  invoke.mockReset().mockImplementation(async (command) => {
    if (command === 'create_project_cmd') return { id: 'new-owner' };
    if (command === 'rename_project_cmd') return null;
    throw new Error('Unexpected command: ' + command);
  });
});
afterEach(cleanup);
function show(ui: ReactNode) {
  return render(
    <LibraryContext.Provider value={context}>
      <StudioActionsContext.Provider
        value={{ select: vi.fn(), run: vi.fn(), context: vi.fn(), drop }}
      >
        {ui}
      </StudioActionsContext.Provider>
    </LibraryContext.Provider>,
  );
}
const destination = { projectId: 'project', folderId: null };
function dialog(kind: 'text' | 'project' | 'folder' | 'youtube', inWorkspace = false) {
  const close = vi.fn();
  show(
    <LibraryCreateDialog
      kind={kind}
      destination={kind === 'text' ? { ...destination, projectId: null } : destination}
      close={close}
      inWorkspace={inWorkspace}
    />,
  );
  return close;
}

it('validates empty names, submits trimmed project names and allows Escape cancellation', async () => {
  const close = dialog('project');
  const submit = screen.getByRole('button', { name: 'Создать' });
  expect(submit.hasAttribute('disabled')).toBe(true);
  fireEvent.change(screen.getByRole('textbox', { name: 'Название' }), { target: { value: '   ' } });
  expect(submit.hasAttribute('disabled')).toBe(true);
  fireEvent.change(screen.getByRole('textbox'), { target: { value: '  Created project  ' } });
  fireEvent.click(submit);
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
  expect(invoke).toHaveBeenCalledWith('create_project_cmd', { title: 'Created project' });
  expect(context.refresh).toHaveBeenCalledOnce();
});

it.each([false, true])(
  'creates text with preserved contents and opens the correct destination (workspace=%s)',
  async (inWorkspace) => {
    const close = dialog('text', inWorkspace);
    fireEvent.change(screen.getByRole('textbox', { name: 'Название' }), {
      target: { value: '  Draft  ' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'Текст' }), {
      target: { value: '  Keep whitespace\n' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Создать' }));
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
    const created = context.state.assets.find((a) => a.name === 'Draft')!;
    expect(created.text).toBe('  Keep whitespace\n');
    if (inWorkspace) expect(context.openTool).toHaveBeenCalledWith('text', created.id);
    else expect(context.openAsset).toHaveBeenCalledWith(created, undefined, null);
  },
);

it('disables cancellation while saving, preserves input on error and exposes a successful retry', async () => {
  let reject!: (reason: unknown) => void;
  invoke.mockReturnValueOnce(
    new Promise((_, no) => {
      reject = no;
    }),
  );
  const close = dialog('project');
  fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Retained' } });
  fireEvent.click(screen.getByRole('button', { name: 'Создать' }));
  expect(screen.getByRole('button', { name: 'Сохраняем…' }).hasAttribute('disabled')).toBe(true);
  fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
  expect(close).not.toHaveBeenCalled();
  reject(new Error('Storage unavailable. Retry.'));
  await screen.findByRole('alert');
  expect(screen.getByRole('alert').textContent).toBe('Storage unavailable. Retry.');
  expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('Retained');
  fireEvent.click(screen.getByRole('button', { name: 'Создать' }));
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
});

it('creates a folder under the selected parent and supports both cancellation controls', async () => {
  const close = vi.fn();
  show(
    <LibraryCreateDialog
      kind="folder"
      destination={{ ...destination, folderId: 'parent' }}
      close={close}
    />,
  );
  fireEvent.change(screen.getByRole('textbox'), { target: { value: '  Child  ' } });
  fireEvent.click(screen.getByRole('button', { name: 'Создать' }));
  await waitFor(() => expect(close).toHaveBeenCalledOnce());
  expect(context.state.folders[0]).toMatchObject({
    projectId: 'project',
    parentId: 'parent',
    name: 'Child',
  });
  fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
  fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
  expect(close).toHaveBeenCalledTimes(3);
});

it('shows YouTube import errors without closing or inventing a published material', async () => {
  invoke.mockRejectedValueOnce({ code: 'VALIDATION', message: 'Unsupported source URL' });
  const close = dialog('youtube');
  fireEvent.change(screen.getByRole('textbox', { name: 'Ссылка YouTube' }), {
    target: { value: 'https://example.com/video' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Создать' }));
  expect((await screen.findByRole('alert')).textContent).toBe('Unsupported source URL');
  expect(close).not.toHaveBeenCalled();
  expect(context.state.assets).toHaveLength(1);
});

it('saves trimmed properties and unique tags, preserving accepted state on a failed save', async () => {
  show(
    <AssetInspector
      asset={context.state.assets[0]}
      browser={context.state.browser}
      select={vi.fn()}
    />,
  );
  fireEvent.change(screen.getByRole('textbox', { name: 'Название' }), {
    target: { value: '  Renamed  ' },
  });
  fireEvent.change(screen.getByRole('textbox', { name: 'Теги через запятую' }), {
    target: { value: 'one, two, one, , ' },
  });
  vi.mocked(context.mutate).mockRejectedValueOnce(new Error('Write failed. Retry saving.'));
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить свойства' }));
  expect((await screen.findByRole('alert')).textContent).toBe('Write failed. Retry saving.');
  expect(context.state.assets[0].name).toBe('asset');
  expect((screen.getByRole('textbox', { name: 'Название' }) as HTMLInputElement).value).toBe(
    '  Renamed  ',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить свойства' }));
  await waitFor(() => expect(context.state.assets[0].name).toBe('Renamed'));
  expect(context.state.assets[0].tags).toEqual(['one', 'two']);
});

it.each(['text', 'subtitle', 'audio', 'video'] as const)(
  'opens available %s tools with the project context',
  async (kind) => {
    const selected = asset('asset', { kind });
    show(
      <AssetInspector
        asset={selected}
        browser={{ ...context.state.browser, scope: 'project', projectId: 'project' }}
        select={vi.fn()}
      />,
    );
    const label =
      kind === 'text' || kind === 'subtitle' ? 'Текст и подготовка' : 'Источник и субтитры';
    fireEvent.click(screen.getByRole('button', { name: label }));
    await waitFor(() =>
      expect(context.openAsset).toHaveBeenCalledWith(
        selected,
        kind === 'text' || kind === 'subtitle' ? 'text' : 'source',
        'project',
      ),
    );
  },
);

it('delegates attachment, removes only the selected membership and confirms global removal', async () => {
  context.state.links = [
    { assetId: 'asset', projectId: 'project', folderId: null },
    { assetId: 'asset', projectId: 'other', folderId: null },
  ];
  context.state.workspaces = [
    { id: 'standalone', projectId: null, assetId: 'asset', tabs: [], activeTab: null },
    {
      id: 'project',
      projectId: 'project',
      assetId: null,
      tabs: [
        { id: 'a', tool: 'text', assetId: 'asset', draft: '{}' },
        { id: 'b', tool: 'results', assetId: 'another', draft: '{}' },
      ],
      activeTab: 'a',
    },
  ];
  const select = vi.fn();
  show(
    <AssetInspector
      asset={context.state.assets[0]}
      browser={{ ...context.state.browser, projectId: 'project' }}
      select={select}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Добавить копию в проект' }));
  expect(drop).toHaveBeenCalledWith('asset', 'project');
  fireEvent.click(screen.getByRole('button', { name: 'Убрать из проекта' }));
  await waitFor(() =>
    expect(context.state.links).toEqual([{ assetId: 'asset', projectId: 'other', folderId: null }]),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Удалить из библиотеки…' }));
  fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
  expect(context.state.hiddenAssets).toEqual([]);
  fireEvent.click(screen.getByRole('button', { name: 'Удалить из библиотеки…' }));
  fireEvent.click(screen.getByRole('button', { name: 'Подтвердить удаление' }));
  await waitFor(() => expect(select).toHaveBeenCalledWith(null));
  expect(context.state.hiddenAssets).toEqual(['asset']);
  expect(context.state.links).toEqual([]);
  expect(context.state.workspaces).toMatchObject([
    { id: 'project', activeTab: 'b', tabs: [{ id: 'b' }] },
  ]);
});

it('keeps unavailable media copies in an actionable failed state', async () => {
  show(
    <AssetInspector
      asset={asset('pending', { kind: 'video', text: null })}
      browser={{ ...context.state.browser, projectId: 'project' }}
      select={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Независимая копия' }));
  expect((await screen.findByRole('alert')).textContent).toContain(
    'Сначала загрузите исходный медиафайл',
  );
  expect(invoke).not.toHaveBeenCalled();
});

it('renders bounded version ancestry, hides removed children and opens a chosen version', async () => {
  const selected = asset('current', { parentId: 'parent' });
  context.state.assets = [
    selected,
    asset('parent', { parentId: 'grandparent' }),
    asset('grandparent', { parentId: 'parent' }),
    asset('child', { parentId: 'current' }),
    asset('hidden-child', { parentId: 'current' }),
  ];
  context.state.hiddenAssets = ['hidden-child'];
  show(<AssetVersions asset={selected} />);
  fireEvent.click(screen.getByText('Версии и производные материалы'));
  const details = within(screen.getByText('Версии и производные материалы').closest('details')!);
  expect(details.getAllByRole('button').map((b) => b.textContent)).toEqual([
    'grandparent',
    'parent',
    'child',
  ]);
  fireEvent.click(details.getByRole('button', { name: 'parent' }));
  await waitFor(() => expect(context.openAsset).toHaveBeenCalledWith(context.state.assets[1]));
});

it('renames a project with its expected revision and keeps errors recoverable', async () => {
  context.state.links = [
    { projectId: 'project', assetId: 'asset', folderId: null },
    { projectId: 'other', assetId: 'other-asset', folderId: null },
  ];
  context.state.folders = [
    { id: 'folder', projectId: 'project', parentId: null, name: 'Removed' },
    { id: 'other-folder', projectId: 'other', parentId: null, name: 'Retained' },
  ];
  context.state.workspaces = [
    { id: 'workspace', projectId: 'project', assetId: null, activeTab: null, tabs: [] },
    { id: 'other-workspace', projectId: 'other', assetId: null, activeTab: null, tabs: [] },
  ];
  const navigate = vi.fn();
  show(
    <LibraryProjectActions
      browser={{ ...context.state.browser, projectId: 'project' }}
      navigate={navigate}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Переименовать проект' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Новое название' }), {
    target: { value: 'Renamed' },
  });
  invoke.mockRejectedValueOnce({ code: 'CONFLICT', message: 'Reload the current revision' });
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить' }));
  expect((await screen.findByRole('alert')).textContent).toBe('Reload the current revision');
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить' }));
  await waitFor(() => expect(context.refresh).toHaveBeenCalledOnce());
  expect(invoke).toHaveBeenLastCalledWith('rename_project_cmd', {
    projectId: 'project',
    title: 'Renamed',
    expectedRevision: 7,
  });
  fireEvent.click(screen.getByRole('button', { name: 'Удалить проект…' }));
  fireEvent.click(screen.getByRole('button', { name: 'Подтвердить удаление раздела' }));
  await waitFor(() =>
    expect(navigate).toHaveBeenCalledWith({ scope: 'all', projectId: null, folderId: null }),
  );
  expect(context.state.removedProjects).toEqual(['project']);
  expect(context.state.links.map((link) => link.projectId)).toEqual(['other']);
  expect(context.state.folders.map((folder) => folder.id)).toEqual(['other-folder']);
  expect(context.state.workspaces.map((workspace) => workspace.id)).toEqual(['other-workspace']);
  expect(context.state.assets[0].text).toBe('Accepted words');
});

it('renames and removes folders while moving children and links to their parent', async () => {
  context.state.folders = [
    { id: 'folder', projectId: 'project', parentId: 'root', name: 'Folder' },
    { id: 'child', projectId: 'project', parentId: 'folder', name: 'Child' },
  ];
  context.state.links = [{ assetId: 'asset', projectId: 'project', folderId: 'folder' }];
  const navigate = vi.fn();
  show(
    <LibraryProjectActions
      browser={{ ...context.state.browser, projectId: 'project', folderId: 'folder' }}
      navigate={navigate}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Переименовать папку' }));
  fireEvent.change(screen.getByRole('textbox'), { target: { value: '  Renamed  ' } });
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить' }));
  await waitFor(() => expect(context.state.folders[0].name).toBe('Renamed'));
  fireEvent.click(screen.getByRole('button', { name: 'Удалить папку…' }));
  fireEvent.click(screen.getByRole('button', { name: 'Подтвердить удаление раздела' }));
  await waitFor(() => expect(navigate).toHaveBeenCalledWith({ folderId: 'root' }));
  expect(context.state.folders).toEqual([
    { id: 'child', projectId: 'project', parentId: 'root', name: 'Child' },
  ]);
  expect(context.state.links[0].folderId).toBe('root');
  expect(invoke).not.toHaveBeenCalled();
});

it('discards cancelled project edits without mutating accepted state', () => {
  show(
    <LibraryProjectActions
      browser={{ ...context.state.browser, projectId: 'project' }}
      navigate={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Переименовать проект' }));
  fireEvent.change(screen.getByRole('textbox'), { target: { value: 'Discarded edit' } });
  fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
  expect(screen.queryByRole('textbox')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Удалить проект…' }));
  fireEvent.click(screen.getByRole('button', { name: 'Отмена' }));
  expect(screen.queryByRole('alertdialog')).toBeNull();
  expect(invoke).not.toHaveBeenCalled();
  expect(context.mutate).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Переименовать проект' }));
  expect(screen.getByRole('textbox')).toHaveProperty('value', 'Project');
});

it('keeps a failed removal available for explicit retry without navigating away', async () => {
  const navigate = vi.fn();
  vi.mocked(context.mutate).mockRejectedValueOnce(new Error('Persistence unavailable'));
  show(
    <LibraryProjectActions
      browser={{ ...context.state.browser, projectId: 'project' }}
      navigate={navigate}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Удалить проект…' }));
  fireEvent.click(screen.getByRole('button', { name: 'Подтвердить удаление раздела' }));
  expect((await screen.findByRole('alert')).textContent).toBe('Persistence unavailable');
  expect(context.state.removedProjects).toEqual([]);
  expect(navigate).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Подтвердить удаление раздела' }));
  await waitFor(() => expect(navigate).toHaveBeenCalledOnce());
  expect(context.state.removedProjects).toEqual(['project']);
});

it('admits only one rename while pending and prevents discarding its submitted input', async () => {
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  invoke.mockImplementationOnce(() => pending);
  show(
    <LibraryProjectActions
      browser={{ ...context.state.browser, projectId: 'project' }}
      navigate={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Переименовать проект' }));
  const input = screen.getByRole('textbox');
  fireEvent.change(input, { target: { value: 'Pending edit' } });
  fireEvent.submit(input.closest('form')!);
  fireEvent.submit(input.closest('form')!);
  try {
    expect(invoke.mock.calls.filter(([command]) => command === 'rename_project_cmd')).toHaveLength(
      1,
    );
    expect(screen.getByRole('button', { name: 'Сохранить' }).hasAttribute('disabled')).toBe(true);
    expect(screen.getByRole('button', { name: 'Отмена' }).hasAttribute('disabled')).toBe(true);
    expect(input).toHaveProperty('value', 'Pending edit');
  } finally {
    await act(async () => release());
  }
  await waitFor(() => expect(context.refresh).toHaveBeenCalledOnce());
  expect(screen.queryByRole('textbox')).toBeNull();
});

it('admits one removal and keeps cancellation unavailable until persistence returns', async () => {
  let release!: () => void;
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  vi.mocked(context.mutate).mockReturnValueOnce(pending);
  const navigate = vi.fn();
  show(
    <LibraryProjectActions
      browser={{ ...context.state.browser, projectId: 'project' }}
      navigate={navigate}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Удалить проект…' }));
  const confirm = screen.getByRole('button', { name: 'Подтвердить удаление раздела' });
  fireEvent.click(confirm);
  fireEvent.click(confirm);
  try {
    expect(context.mutate).toHaveBeenCalledOnce();
    expect(confirm.hasAttribute('disabled')).toBe(true);
    expect(screen.getByRole('button', { name: 'Отмена' }).hasAttribute('disabled')).toBe(true);
    expect(navigate).not.toHaveBeenCalled();
  } finally {
    await act(async () => release());
  }
  await waitFor(() => expect(navigate).toHaveBeenCalledOnce());
});
