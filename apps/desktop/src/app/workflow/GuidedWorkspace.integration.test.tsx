import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { NavigationProvider, useNavigation } from '@/shared/router';
import { useProjectContext } from '@/entities/project';
import { GuidedWorkspace } from './GuidedWorkspace';
import App from '../../App';
import {
  backend,
  deferred,
  dialogSupport,
  LibraryHarness,
  ProjectHarness,
} from '../../../tests/studio';
const api = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  toast: vi.fn(),
  check: vi.fn(),
}));
vi.mock('@/shared/api/tauri', () => ({ invoke: api.invoke, listen: api.listen }));
vi.mock('@/shared/ui/toast', () => ({ toast: { default: api.toast } }));
vi.mock('@/features/app-update/api/appUpdateClient', () => ({
  nativeAppUpdateClient: {
    isSupported: () => true,
    getCurrentVersion: async () => '0.1.0',
    check: api.check,
    relaunch: vi.fn(),
  },
}));
let data: ReturnType<typeof backend>;
const update = {
  currentVersion: '0.1.0',
  version: '0.2.0',
  close: async () => {},
  downloadAndInstall: async () => {},
};
beforeEach(() => {
  vi.resetAllMocks();
  dialogSupport();
  data = backend();
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
  api.check.mockResolvedValue(update);
});
afterEach(cleanup);
function Controls() {
  const { setCurrentView, currentView } = useNavigation();
  const { setProject } = useProjectContext();
  return (
    <>
      <button onClick={() => setCurrentView('project')}>Route project</button>
      <button onClick={() => setProject(null)}>Remove selected project</button>
      <span data-testid="route">{currentView}</span>
    </>
  );
}
const shell = (children: React.ReactNode) => (
  <LibraryHarness>
    <NavigationProvider>
      <ProjectHarness>{children}</ProjectHarness>
    </NavigationProvider>
  </LibraryHarness>
);
it('returns from settings to the selected workspace and retains project context', async () => {
  render(
    shell(
      <>
        <Controls />
        <GuidedWorkspace onOpenLibrary={vi.fn()} />
      </>,
    ),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Route project' }));
  await screen.findByLabelText('Project workspace', {}, { timeout: 4000 });
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  await screen.findByRole('heading', { name: 'Настройки', level: 1 });
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  await screen.findByLabelText('Project workspace', {}, { timeout: 4000 });
  expect(screen.getByTestId('route').textContent).toBe('project');
});
it.each(['project', 'settings'])(
  'returns home when the selected project is removed in %s',
  async (route) => {
    render(
      shell(
        <>
          <Controls />
          <GuidedWorkspace onOpenLibrary={vi.fn()} />
        </>,
      ),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Route project' }));
    await screen.findByLabelText('Project workspace', {}, { timeout: 4000 });
    if (route === 'settings') {
      fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
      await screen.findByRole('heading', { name: 'Настройки', level: 1 });
    }
    fireEvent.click(screen.getByRole('button', { name: 'Remove selected project' }));
    await screen.findByRole('heading', { name: 'Projects', level: 1 });
    expect(screen.getByTestId('route').textContent).toBe('home');
  },
);
it('routes a closed selection to home without rendering a stale workspace', async () => {
  render(
    <LibraryHarness>
      <NavigationProvider>
        <ProjectHarness selected={null}>
          <Controls />
          <GuidedWorkspace onOpenLibrary={vi.fn()} />
        </ProjectHarness>
      </NavigationProvider>
    </LibraryHarness>,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Route project' }));
  expect(screen.queryByLabelText('Project workspace')).toBeNull();
  await waitFor(() => expect(screen.getByTestId('route').textContent).toBe('home'));
});
it('opens update settings through the workflow event and removes its listener on unmount', async () => {
  const rendered = render(shell(<GuidedWorkspace onOpenLibrary={vi.fn()} />));
  act(() => window.dispatchEvent(new Event('auralis-workflow-settings')));
  await screen.findByRole('heading', { name: 'Настройки', level: 1 });
  rendered.unmount();
  act(() => window.dispatchEvent(new Event('auralis-workflow-settings')));
  expect(screen.queryByRole('heading')).toBeNull();
});
it('keeps one update notification owner and opens the current view from an earlier library notification', async () => {
  render(shell(<App />));
  await waitFor(() => expect(api.toast).toHaveBeenCalledTimes(1));
  const action = api.toast.mock.calls[0][1].action.onClick;
  fireEvent.click(screen.getByRole('button', { name: 'Пошаговая переозвучка' }));
  await screen.findByRole('heading', { name: 'Projects', level: 1 });
  act(() => action());
  await screen.findByRole('heading', { name: 'Настройки', level: 1 });
  expect(api.toast).toHaveBeenCalledTimes(1);
  expect(api.check).toHaveBeenCalledTimes(1);
});
it('opens library settings from a notification created while the guided view was active', async () => {
  const pending = deferred<typeof update>();
  api.check.mockReturnValue(pending.promise);
  render(shell(<App />));
  fireEvent.click(screen.getByRole('button', { name: 'Пошаговая переозвучка' }));
  await screen.findByRole('heading', { name: 'Projects', level: 1 });
  await act(async () => pending.resolve(update));
  await waitFor(() => expect(api.toast).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole('button', { name: 'Медиабиблиотека' }));
  await screen.findByRole('heading', { name: 'Все материалы' });
  act(() => api.toast.mock.calls[0][1].action.onClick());
  expect(await screen.findByRole('dialog', { name: 'Настройки' })).toBeTruthy();
  expect(api.check).toHaveBeenCalledTimes(1);
});
