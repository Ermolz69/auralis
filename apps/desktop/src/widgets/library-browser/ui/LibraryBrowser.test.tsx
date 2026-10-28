import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import {
  LibraryContext,
  StudioActionsContext,
  type LibraryContextValue,
  type StudioActions,
} from '@/entities/library';
import { emptyLibrary, type LibraryAsset } from '@/shared/api/contracts/library';
import { LibraryBrowser } from './LibraryBrowser';

vi.mock('@/shared/api/tauri', () => ({ invoke: vi.fn(), listen: vi.fn() }));
const asset = (id: string, name: string, extra: Partial<LibraryAsset> = {}): LibraryAsset => ({
  id,
  name,
  ownerProjectId: 'owner',
  kind: 'text',
  artifactId: null,
  format: 'txt',
  text: 'Words',
  parentId: null,
  tags: [],
  openedAt: null,
  ...extra,
});
let context: LibraryContextValue;
let actions: StudioActions;
beforeEach(() => {
  const state = emptyLibrary();
  state.assets = [
    asset('a', 'Alpha', { tags: ['Русский'], openedAt: '2026-10-27' }),
    asset('b', 'Beta', { kind: 'subtitle', format: 'srt' }),
    asset('hidden', 'Hidden'),
  ];
  state.hiddenAssets = ['hidden'];
  state.links = [{ assetId: 'a', projectId: 'project', folderId: null }];
  state.standaloneOwners = ['owner'];
  state.removedProjects = ['removed'];
  context = {
    state,
    projects: [
      { id: 'project', title: 'Project', revision: 3 },
      { id: 'owner', title: 'Internal owner' },
      { id: 'removed', title: 'Removed' },
    ] as LibraryContextValue['projects'],
    loading: false,
    error: null,
    view: 'library',
    workspaceId: null,
    setView: vi.fn(),
    activateWorkspace: vi.fn(),
    mutate: vi.fn().mockResolvedValue(undefined),
    refresh: vi.fn().mockResolvedValue(undefined),
    openAsset: vi.fn().mockResolvedValue(undefined),
    openProject: vi.fn().mockResolvedValue(undefined),
    openTool: vi.fn().mockResolvedValue(undefined),
  };
  actions = {
    select: vi.fn(),
    run: vi.fn(),
    context: vi.fn((event) => event.stopPropagation()),
    drop: vi.fn(),
  };
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

function show() {
  return render(
    <LibraryContext.Provider value={context}>
      <StudioActionsContext.Provider value={actions}>
        <LibraryBrowser />
      </StudioActionsContext.Provider>
    </LibraryContext.Provider>,
  );
}
function cards() {
  return within(screen.getByRole('list', { name: 'Материалы библиотеки' }));
}

it('filters by case-insensitive tags and material kind using visible controls', () => {
  show();
  expect(cards().getAllByRole('listitem')).toHaveLength(2);
  expect(screen.queryByRole('button', { name: /Hidden/ })).toBeNull();
  fireEvent.change(screen.getByRole('searchbox', { name: 'Поиск материалов' }), {
    target: { value: 'РУССКИЙ' },
  });
  expect(cards().getAllByRole('listitem')).toHaveLength(1);
  expect(cards().getByRole('button', { name: /Alpha/ })).toBeTruthy();
  fireEvent.change(screen.getByRole('combobox', { name: 'Тип материала' }), {
    target: { value: 'subtitle' },
  });
  expect(screen.getByText('Здесь пока нет материалов.')).toBeTruthy();
  fireEvent.change(screen.getByRole('searchbox'), { target: { value: '' } });
  expect(cards().getByRole('button', { name: /Beta/ })).toBeTruthy();
});

it('selects and opens through click, double-click and Enter with the active project target', () => {
  show();
  fireEvent.click(screen.getByRole('button', { name: 'Project' }));
  const card = cards().getByRole('button', { name: /Alpha/ });
  fireEvent.click(card);
  expect(card.getAttribute('aria-pressed')).toBe('true');
  expect(actions.select).toHaveBeenCalledWith({ projectId: 'project', assetId: 'a' });
  fireEvent.doubleClick(card);
  fireEvent.keyDown(card, { key: 'Enter' });
  expect(context.openAsset).toHaveBeenCalledTimes(2);
  expect(context.openAsset).toHaveBeenLastCalledWith(context.state.assets[0], undefined, 'project');
});

it('forwards asset and background context menus and keyboard context actions', () => {
  show();
  const card = cards().getByRole('button', { name: /Alpha/ });
  fireEvent.contextMenu(card);
  expect(actions.context).toHaveBeenCalledWith(expect.anything(), {
    projectId: null,
    assetId: 'a',
  });
  fireEvent.keyDown(card, { key: 'ContextMenu' });
  fireEvent.keyDown(card, { key: 'F10', shiftKey: true });
  expect(actions.context).toHaveBeenCalledTimes(3);
  fireEvent.keyDown(card, { key: 'ArrowRight' });
  expect(actions.context).toHaveBeenCalledTimes(3);
  fireEvent.contextMenu(screen.getByRole('region', { name: 'Библиотека материалов' }));
  expect(actions.context).toHaveBeenLastCalledWith(expect.anything(), {
    projectId: null,
    assetId: null,
  });
});

it('navigates scope and project kinds and admits only known asset drops', () => {
  show();
  expect(screen.queryByRole('button', { name: 'Internal owner' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Removed' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Без проекта' }));
  expect(cards().getByRole('button', { name: /Beta/ })).toBeTruthy();
  expect(cards().queryByRole('button', { name: /Alpha/ })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Недавние' }));
  expect(cards().getByRole('button', { name: /Alpha/ })).toBeTruthy();
  const project = screen.getByRole('button', { name: 'Project' });
  const transfer = {
    setData: vi.fn(),
    getData: vi.fn().mockReturnValue('a'),
    types: ['application/x-auralis-asset'],
    dropEffect: '',
  };
  fireEvent.dragStart(cards().getByRole('button', { name: /Alpha/ }), { dataTransfer: transfer });
  expect(transfer.setData).toHaveBeenCalledWith('application/x-auralis-asset', 'a');
  fireEvent.dragOver(project, { dataTransfer: transfer });
  expect(transfer.dropEffect).toBe('copy');
  fireEvent.dragLeave(project);
  fireEvent.drop(project, { dataTransfer: transfer });
  expect(actions.drop).toHaveBeenCalledWith('a', 'project');
  transfer.getData.mockReturnValue('foreign');
  fireEvent.drop(project, { dataTransfer: transfer });
  expect(actions.drop).toHaveBeenCalledOnce();
  fireEvent.dragOver(project, {
    dataTransfer: { ...transfer, types: ['text/plain'], dropEffect: '' },
  });
  fireEvent.click(project);
  fireEvent.click(within(screen.getByRole('navigation')).getByRole('button', { name: 'Субтитры' }));
  expect(screen.getByText('Здесь пока нет материалов.')).toBeTruthy();
  fireEvent.doubleClick(project);
  fireEvent.keyDown(project, { key: 'Enter' });
  expect(context.openProject).toHaveBeenCalledTimes(2);
  fireEvent.contextMenu(project);
  fireEvent.keyDown(project, { key: 'F10', shiftKey: true });
  expect(actions.context).toHaveBeenLastCalledWith(expect.anything(), {
    projectId: 'project',
    assetId: null,
  });
  fireEvent.click(screen.getByRole('button', { name: 'Создать проект' }));
  expect(actions.run).toHaveBeenCalledWith('new-project');
});

it('persists the last browser state once and cancels persistence after unmount', async () => {
  vi.useFakeTimers();
  const view = show();
  fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'First' } });
  fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'Last' } });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(350);
  });
  expect(context.mutate).toHaveBeenCalledOnce();
  const change = vi.mocked(context.mutate).mock.calls[0][0];
  change(context.state);
  expect(context.state.browser.search).toBe('Last');
  fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'Discarded' } });
  view.unmount();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(500);
  });
  expect(context.mutate).toHaveBeenCalledOnce();
});

it('shows loading, hydrates the saved browser and clears removed project context', () => {
  context.loading = true;
  const view = show();
  expect(screen.getByRole('status').textContent).toBe('Загружаем библиотеку…');
  context = {
    ...context,
    loading: false,
    state: {
      ...context.state,
      browser: {
        ...context.state.browser,
        scope: 'project',
        projectId: 'project',
        selectedId: null,
      },
    },
  };
  view.rerender(
    <LibraryContext.Provider value={context}>
      <LibraryBrowser />
    </LibraryContext.Provider>,
  );
  expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Project');
  context = { ...context, projects: [], state: { ...context.state, removedProjects: ['project'] } };
  view.rerender(
    <LibraryContext.Provider value={context}>
      <LibraryBrowser />
    </LibraryContext.Provider>,
  );
  expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Все материалы');
});
