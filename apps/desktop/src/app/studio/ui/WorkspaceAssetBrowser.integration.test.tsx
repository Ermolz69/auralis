import { useState } from 'react';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useLibrary } from '@/entities/library';
import { WorkspaceAssetBrowser } from './WorkspaceAssetBrowser';
import { StudioCommands } from './StudioCommands';
import {
  asset,
  backend,
  dialogSupport,
  LibraryHarness,
  project,
  snapshot,
} from '../../../../tests/studio';

const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => api);
let data: ReturnType<typeof backend>;

beforeEach(() => {
  vi.resetAllMocks();
  dialogSupport();
  vi.stubGlobal(
    'IntersectionObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  data = backend();
  data.state.assets = [
    { ...asset, tags: ['Русский'] },
    { ...asset, id: 'subtitle', name: 'Interview subtitles', kind: 'subtitle', format: 'srt' },
    { ...asset, id: 'audio', name: 'Ready audio', kind: 'audio', format: 'wav' },
    { ...asset, id: 'video', name: 'Source video', kind: 'video', format: 'mp4' },
    { ...asset, id: 'foreign', name: 'Foreign material', ownerProjectId: 'other' },
    { ...asset, id: 'hidden', name: 'Hidden material' },
  ];
  data.state.links = data.state.assets.map((item) => ({
    projectId: item.id === 'foreign' ? 'other' : project.id,
    assetId: item.id,
    folderId: null,
  }));
  data.state.hiddenAssets = ['hidden'];
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function Browser() {
  const { loading, openAsset } = useLibrary();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  if (loading) return null;
  return (
    <WorkspaceAssetBrowser
      projectId={project.id}
      selectedId={selectedId}
      onSelect={(item) => setSelectedId(item.id)}
      onOpen={(item) => void openAsset(item, undefined, project.id)}
    />
  );
}
async function show() {
  render(
    <LibraryHarness>
      <StudioCommands>
        <Browser />
      </StudioCommands>
    </LibraryHarness>,
  );
  await screen.findByRole('searchbox', { name: 'Поиск в материалах проекта' });
}
const saved = () => snapshot(screen.getByLabelText('Library snapshot'));

it('filters current-project materials by name and tags without exposing hidden or foreign entries', async () => {
  await show();
  const list = within(screen.getByRole('list', { name: 'Содержимое проекта' }));
  expect(list.getAllByRole('button')).toHaveLength(4);
  expect(screen.queryByText('Foreign material')).toBeNull();
  expect(screen.queryByText('Hidden material')).toBeNull();
  const search = screen.getByRole('searchbox');
  fireEvent.change(search, { target: { value: ' РУССКИЙ ' } });
  expect(list.getAllByRole('button')).toHaveLength(1);
  fireEvent.click(list.getByRole('button', { name: /First text/ }));
  expect(list.getByRole('button', { name: /First text/ }).getAttribute('aria-current')).toBe(
    'page',
  );
  fireEvent.change(search, { target: { value: 'SUBTITLES' } });
  expect(list.getByRole('button', { name: /Interview subtitles/ })).toBeTruthy();
  fireEvent.change(search, { target: { value: 'no matching material' } });
  expect(list.queryAllByRole('button')).toHaveLength(0);
  expect(screen.getByText('Материалы не найдены.')).toBeTruthy();
  fireEvent.change(search, { target: { value: '' } });
  expect(list.getAllByRole('button')).toHaveLength(4);
});

it('opens exact materials through double click and Enter in the same owning workspace', async () => {
  await show();
  fireEvent.doubleClick(screen.getByRole('button', { name: /Interview subtitles/ }));
  await waitFor(() => expect(saved().workspaces[0]?.tabs[0]?.assetId).toBe('subtitle'));
  expect(saved().workspaces[0].projectId).toBe(project.id);
  fireEvent.keyDown(screen.getByRole('button', { name: /Ready audio/ }), { key: 'Enter' });
  await waitFor(() => expect(saved().workspaces[0].tabs).toHaveLength(2));
  expect(saved().workspaces).toHaveLength(1);
  expect(saved().workspaces[0].tabs[1]).toMatchObject({ tool: 'results', assetId: 'audio' });
  expect(saved().assets.find((item) => item.id === 'audio')?.openedAt).toBeTruthy();
  expect(api.invoke).toHaveBeenCalledWith('validate_media_project_cmd', { projectId: project.id });
});

it('uses keyboard and pointer context menus for the chosen material and restores focus on Escape', async () => {
  await show();
  const material = screen.getByRole('button', { name: /First text/ });
  material.focus();
  fireEvent.keyDown(material, { key: ' ' });
  expect(screen.queryByRole('menu')).toBeNull();
  fireEvent.keyDown(material, { key: 'F10', shiftKey: true });
  const menu = await screen.findByRole('menu', { name: 'Действия' });
  fireEvent.keyDown(menu, { key: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
  expect(document.activeElement).toBe(material);
  fireEvent.contextMenu(material);
  fireEvent.click(await screen.findByRole('menuitem', { name: 'Свойства…' }));
  const dialog = await screen.findByRole('dialog', { name: 'Свойства материала' });
  expect(dialog.textContent).toContain('First text · Русский');
  expect(dialog.textContent).toContain(project.title);
  fireEvent.click(within(dialog).getByRole('button', { name: 'Закрыть диалог' }));
});

it('offers project actions from empty space without retaining a previously selected material', async () => {
  await show();
  fireEvent.contextMenu(screen.getByRole('button', { name: /First text/ }));
  fireEvent.keyDown(await screen.findByRole('menu'), { key: 'Escape' });
  fireEvent.contextMenu(screen.getByRole('list', { name: 'Содержимое проекта' }));
  const menu = await screen.findByRole('menu', { name: 'Действия' });
  expect(within(menu).queryByRole('menuitem', { name: 'Свойства…' })).toBeNull();
  fireEvent.click(within(menu).getByRole('menuitem', { name: 'Новый текст…' }));
  const dialog = await screen.findByRole('dialog');
  expect(dialog.textContent).toContain(project.title);
});

it('keeps the empty-project state distinct from a search miss', async () => {
  data.state.links = [];
  await show();
  expect(screen.getByText('В проекте пока нет материалов.')).toBeTruthy();
  expect(screen.queryByText('Материалы не найдены.')).toBeNull();
});
