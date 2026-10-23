// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
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
    if (command === 'get_library_cmd') return structuredClone(state);
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
  fireEvent.click(screen.getByRole('button', { name: 'Синтез речи' }));
  await screen.findByRole('tab', { name: /Синтез речи/ });
  fireEvent.click(screen.getByRole('button', { name: 'Библиотека' }));
  expect(screen.queryByRole('tablist')).toBeNull();
  fireEvent.change(screen.getByRole('combobox', { name: 'Открытые рабочие пространства' }), {
    target: { value: 'asset:a' },
  });
  expect(await screen.findByRole('tab', { name: /Синтез речи/ })).toBeTruthy();
  expect(state.workspaces[0].tabs).toHaveLength(2);
});
it('keeps settings separate and recovers when the open project is removed elsewhere', async () => {
  render(<App />);
  fireEvent.doubleClick(await screen.findByRole('button', { name: 'Project one' }));
  await screen.findByRole('tablist');
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  expect(await screen.findByRole('heading', { name: 'Settings page' })).toBeTruthy();
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
  const jobs = {
    jobs: { job1: { status: 'running', title: 'Global job', progress: { percent: 42 } } },
  };
  render(
    <JobContext.Provider value={jobs as unknown as React.ContextType<typeof JobContext>}>
      <App />
    </JobContext.Provider>,
  );
  expect(await screen.findByRole('button', { name: 'Очередь · 1' })).toBeTruthy();
  fireEvent.doubleClick(screen.getByRole('button', { name: 'Project one' }));
  await screen.findByRole('tablist');
  expect(screen.getByText('Global job · 42%')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Библиотека' }));
  expect(screen.getByRole('button', { name: 'Очередь · 1' })).toBeTruthy();
});
