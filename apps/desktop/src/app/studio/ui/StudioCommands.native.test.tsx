import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { LibraryProvider, useStudioActions } from '@/entities/library';
import { StudioCommands } from './StudioCommands';
import { StudioTitlebar } from './StudioTitlebar';
import { asset, backend, deferred, project } from '../../../../tests/studio';
const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
const native = vi.hoisted(() => ({
  isMaximized: vi.fn(),
  onResized: vi.fn(),
  minimize: vi.fn(),
  toggleMaximize: vi.fn(),
  close: vi.fn(),
  stop: vi.fn(),
}));
vi.mock('@/shared/api/tauri', () => api);
vi.mock('@tauri-apps/api/core', async (original) => ({
  ...(await original<typeof import('@tauri-apps/api/core')>()),
  isTauri: () => true,
}));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => native }));
let data: ReturnType<typeof backend>;
beforeEach(() => {
  vi.resetAllMocks();
  data = backend();
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
  native.isMaximized.mockResolvedValue(false);
  native.onResized.mockResolvedValue(native.stop);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});
function Drop() {
  const actions = useStudioActions();
  return <button onClick={() => actions.drop(asset.id, project.id)}>Start copy</button>;
}
const setup = () =>
  render(
    <LibraryProvider>
      <StudioCommands>
        <StudioTitlebar />
        <Drop />
      </StudioCommands>
    </LibraryProvider>,
  );
it('reports File Exit failures in the command alert and admits a subsequent retry', async () => {
  native.close
    .mockRejectedValueOnce(new Error('Window close refused'))
    .mockResolvedValue(undefined);
  setup();
  const exit = () => {
    fireEvent.click(screen.getByRole('menuitem', { name: 'Файл' }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'Выход' }));
  };
  exit();
  expect((await screen.findByRole('alert')).textContent).toContain('Window close refused');
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть' }));
  expect(screen.queryByRole('alert')).toBeNull();
  exit();
  await waitFor(() => expect(native.close).toHaveBeenCalledTimes(2));
  expect(screen.queryByRole('alert')).toBeNull();
});
it('uses titlebar controls and exposes native operation failures', async () => {
  native.minimize.mockRejectedValueOnce(new Error('Minimize refused'));
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'Свернуть окно' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Minimize refused');
  native.isMaximized.mockResolvedValue(true);
  fireEvent.click(screen.getByRole('button', { name: 'Развернуть окно' }));
  await screen.findByRole('button', { name: 'Восстановить окно' });
  expect(native.toggleMaximize).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть окно' }));
  await waitFor(() => expect(native.close).toHaveBeenCalledTimes(1));
});
it('releases the copy observer on unmount without cancelling the backend operation', async () => {
  vi.useFakeTimers();
  const pending = deferred<unknown>();
  data.overrides.attach_library_asset_cmd = () => pending.promise;
  data.overrides.get_media_operation_cmd = ({ operationId }) => ({
    id: operationId,
    projectId: project.id,
    assetId: asset.id,
    status: 'copying',
    completedBytes: 1,
    totalBytes: 2,
    error: null,
  });
  const rendered = setup();
  await act(async () => {});
  fireEvent.click(screen.getByRole('button', { name: 'Start copy' }));
  await act(async () => vi.advanceTimersByTime(500));
  const polls = api.invoke.mock.calls.filter(
    ([command]) => command === 'get_media_operation_cmd',
  ).length;
  expect(polls).toBe(1);
  rendered.unmount();
  await act(async () => vi.advanceTimersByTime(1000));
  expect(
    api.invoke.mock.calls.filter(([command]) => command === 'get_media_operation_cmd'),
  ).toHaveLength(polls);
  await act(async () => pending.resolve(data.state));
  expect(api.invoke.mock.calls.some(([command]) => command === 'cancel_media_operation_cmd')).toBe(
    false,
  );
});
