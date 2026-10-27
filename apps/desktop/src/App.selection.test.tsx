// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { invoke, listen } from '@/shared/api/tauri';
import { emptyLibrary, type LibraryState } from '@/shared/api/contracts/library';
import { JobContext } from '@/entities/job';
import App from './App';

vi.mock('@/shared/api/tauri', () => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('./features/app-update', () => ({
  AppUpdateProvider: ({ children }: { children: React.ReactNode }) => children,
  AppUpdateNotifier: () => null,
}));
vi.mock('./pages/settings', () => ({ SettingsPage: () => <h1>Settings page</h1> }));
vi.mock('./app/studio/ui/StudioToolPanel', () => ({
  StudioToolPanel: ({ tab }: { tab: { tool: string } }) => <p>Tool: {tab.tool}</p>,
}));
let state: LibraryState;
let projectEvents: Array<() => void>;
const project = {
  id: 'project-1',
  revision: 1,
  title: 'Project one',
  status: 'draft',
  source: null,
  metadata: null,
  createdAt: '2026-10-23T10:00:00Z',
  updatedAt: '2026-10-23T10:00:00Z',
};
let projects: (typeof project)[];
beforeEach(() => {
  vi.resetAllMocks();
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  state = emptyLibrary();
  projectEvents = [];
  projects = [project];
  state.assets.push({
    id: 'a',
    ownerProjectId: project.id,
    name: 'Standalone',
    kind: 'text',
    artifactId: null,
    format: 'txt',
    text: 'Текст',
    parentId: null,
    tags: [],
    openedAt: null,
  });
  vi.mocked(listen).mockImplementation(async (name, callback) => {
    if (name === 'project-updated') projectEvents.push(callback as () => void);
    return () => {};
  });
  vi.mocked(invoke).mockImplementation((async (
    command: string,
    args?: { state?: LibraryState },
  ) => {
    if (command === 'get_library_cmd') {
      const catalog = structuredClone(state);
      catalog.workspaces = catalog.workspaces.filter(
        (workspace) =>
          !workspace.projectId || projects.some((project) => project.id === workspace.projectId),
      );
      return catalog;
    }
    if (command === 'validate_media_project_cmd') return null;
    if (command === 'list_projects_cmd') return projects;
    if (command === 'save_library_cmd') {
      state = structuredClone(args!.state!);
      state.revision++;
      return structuredClone(state);
    }
    throw new Error(`Unexpected command: ${command}`);
  }) as typeof invoke);
});
afterEach(() => cleanup());
it('opens standalone assets outside the library and preserves tabs on return', async () => {
  render(<App />);
  fireEvent.doubleClick(await screen.findByRole('button', { name: /Standalone Текст/ }));
  expect(await screen.findByRole('tab', { name: /Текст и подготовка/ })).toBeTruthy();
  expect(screen.queryByRole('heading', { name: 'Все материалы' })).toBeNull();
  fireEvent.click(screen.getByRole('menuitem', { name: 'Инструменты' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Синтез речи' }));
  await screen.findByRole('tab', { name: /Синтез речи/ });
  fireEvent.click(screen.getByRole('button', { name: 'Auralis — библиотека' }));
  expect(screen.queryByRole('tablist')).toBeNull();
  fireEvent.doubleClick(await screen.findByRole('button', { name: /Standalone Текст/ }));
  expect(await screen.findByRole('tab', { name: /Синтез речи/ })).toBeTruthy();
  expect(state.workspaces[0].tabs).toHaveLength(2);
});
it('opens the selected project material through the tab menu without replacing other tabs', async () => {
  state.assets.push({ ...state.assets[0], id: 'b', name: 'Second material' });
  state.links.push(
    ...['a', 'b'].map((assetId) => ({ projectId: project.id, assetId, folderId: null })),
  );
  render(<App />);
  fireEvent.doubleClick(await screen.findByRole('button', { name: 'Project one' }));
  const resources = await screen.findByRole('complementary', {
    name: 'Материалы текущего проекта',
  });
  fireEvent.click(within(resources).getByRole('button', { name: /Standalone/ }));
  expect(screen.queryByRole('tab')).toBeNull();
  const add = screen.getByRole('menuitem', { name: 'Инструменты' });
  add.focus();
  fireEvent.click(add);
  const menu = screen.getByRole('menu');
  const source = within(menu).getByRole('menuitem', { name: /Источник и субтитры/ });
  expect(source.getAttribute('aria-disabled')).toBe('true');
  expect(document.activeElement).toBe(source);
  fireEvent.keyDown(menu, { key: 'ArrowDown' });
  expect(document.activeElement).toBe(
    within(menu).getByRole('menuitem', { name: 'Текст и подготовка' }),
  );
  fireEvent.keyDown(menu, { key: 'Escape' });
  expect(screen.queryByRole('menu')).toBeNull();
  await waitFor(() => expect(document.activeElement).toBe(add));
  fireEvent.click(add);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Текст и подготовка' }));
  await screen.findByRole('tab', { name: 'Текст и подготовка · Standalone' });
  fireEvent.click(within(resources).getByRole('button', { name: /Second material/ }));
  expect(screen.getAllByRole('tab')).toHaveLength(1);
  fireEvent.click(add);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Синтез речи' }));
  await screen.findByRole('tab', { name: 'Синтез речи · Second material' });
  expect(state.workspaces[0].tabs.map((tab) => [tab.tool, tab.assetId])).toEqual([
    ['text', 'a'],
    ['speech', 'b'],
  ]);
  fireEvent.click(add);
  fireEvent.click(screen.getByRole('menuitem', { name: 'Синтез речи' }));
  await waitFor(() => expect(screen.getAllByRole('tab')).toHaveLength(2));
});
it('keeps settings separate and recovers when the open project is removed elsewhere', async () => {
  render(<App />);
  fireEvent.doubleClick(await screen.findByRole('button', { name: 'Project one' }));
  await screen.findByRole('tablist');
  fireEvent.click(screen.getByRole('button', { name: 'Настройки' }));
  expect(await screen.findByRole('heading', { name: 'Settings page' })).toBeTruthy();
  expect(screen.getByRole('tablist')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть настройки' }));
  projects = [];
  state.workspaces = [];
  await act(async () => projectEvents.forEach((notify) => notify()));
  expect(await screen.findByRole('heading', { name: 'Все материалы' })).toBeTruthy();
});
it('does not publish a broken catalog and exposes a retry after unavailable storage', async () => {
  vi.mocked(invoke).mockRejectedValueOnce({
    code: 'STORAGE',
    message: 'Library storage unavailable',
  });
  render(<App />);
  expect(await screen.findByRole('alert')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Повторить подключение' }));
  expect(await screen.findByRole('button', { name: /Standalone Текст/ })).toBeTruthy();
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
});
it('keeps the global queue outside workspace selection', async () => {
  projects.push({ ...project, id: 'project-2', title: 'Project two' });
  const jobs = {
    jobs: {
      job1: {
        id: 'job1',
        projectId: project.id,
        status: 'pending',
        title: 'Global job',
        progress: { percent: 0 },
      },
    },
  };
  render(
    <JobContext.Provider value={jobs as unknown as React.ContextType<typeof JobContext>}>
      <App />
    </JobContext.Provider>,
  );
  const queue = screen.getByRole('region', { name: 'Текущие процессы' });
  expect(within(queue).getByText('Global job')).toBeTruthy();
  expect(within(queue).getByText('В очереди')).toBeTruthy();
  fireEvent.doubleClick(await screen.findByRole('button', { name: 'Project one' }));
  await screen.findByRole('tablist');
  expect(queue.textContent).toContain('Global job');
  fireEvent.click(screen.getByRole('button', { name: 'Auralis — библиотека' }));
  const second = await screen.findByRole('button', { name: 'Project two' });
  fireEvent.keyDown(second, { key: 'Enter' });
  await screen.findByRole('region', { name: 'Рабочее пространство Project two' });
  expect(queue.textContent).toContain('Global job');
  fireEvent.click(screen.getByRole('button', { name: 'Настройки' }));
  await screen.findByRole('heading', { name: 'Settings page' });
  expect(queue.textContent).toContain('Global job');
  expect(within(queue).queryByRole('button')).toBeNull();
  expect(screen.queryByRole('button', { name: /Очередь|^Библиотека$/ })).toBeNull();
  expect(screen.queryByRole('combobox', { name: 'Открытые рабочие пространства' })).toBeNull();
});
