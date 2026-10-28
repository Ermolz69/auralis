import { useEffect } from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useLibrary } from '@/entities/library';
import { StudioWorkspace } from './StudioWorkspace';
import { StudioCommands, StudioMenuBar } from './StudioCommands';
import { asset, backend, dialogSupport, LibraryHarness, snapshot } from '../../../../tests/studio';
const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => api);
let data: ReturnType<typeof backend>;
beforeEach(() => {
  vi.resetAllMocks();
  dialogSupport();
  data = backend();
  data.state.workspaces = [
    {
      id: 'workspace',
      projectId: null,
      assetId: asset.id,
      activeTab: 'one',
      tabs: [{ id: 'one', tool: 'text', assetId: asset.id, draft: '{}' }],
    },
  ];
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
});
afterEach(async () => {
  await act(async () => cleanup());
  vi.restoreAllMocks();
});
function Workspace() {
  const { loading, state, view, activateWorkspace } = useLibrary();
  useEffect(() => {
    if (!loading) activateWorkspace('workspace');
  }, [loading]);
  return state.workspaces[0] && view === 'workspace' ? (
    <>
      <StudioMenuBar />
      <StudioWorkspace workspace={state.workspaces[0]} />
    </>
  ) : null;
}
async function setup() {
  const view = render(
    <LibraryHarness>
      <StudioCommands>
        <Workspace />
      </StudioCommands>
    </LibraryHarness>,
  );
  await screen.findAllByRole('tab');
  return view;
}
const saved = () => snapshot(screen.getByLabelText('Library snapshot')).workspaces[0];
it('asks before closing a live edit that has not reached the draft debounce', async () => {
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  await setup();
  const input = await screen.findByRole('textbox', { name: 'Русский текст для озвучки' });
  fireEvent.change(input, { target: { value: 'Live edit before persistence' } });
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть Текст и подготовка' }));
  expect(confirm).toHaveBeenCalledTimes(1);
  expect(input).toHaveProperty('value', 'Live edit before persistence');
  expect(saved().tabs).toHaveLength(1);
});
it('flushes a live draft when leaving the workspace before its debounce', async () => {
  const view = await setup();
  const input = await screen.findByRole('textbox', { name: 'Русский текст для озвучки' });
  fireEvent.change(input, { target: { value: 'Draft retained when leaving the workspace' } });
  view.unmount();
  await waitFor(() =>
    expect(JSON.parse(data.state.workspaces[0].tabs[0].draft).text).toBe(
      'Draft retained when leaving the workspace',
    ),
  );
  expect(api.invoke.mock.calls.filter(([command]) => command === 'save_library_cmd')).toHaveLength(
    1,
  );
});
it.each(['Unsaved edit', ''])(
  'requires explicit discard of draft %j and retains it when declined',
  async (text) => {
    data.state.workspaces[0].tabs[0].draft = JSON.stringify({ text });
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    await setup();
    fireEvent.click(screen.getByRole('button', { name: 'Закрыть Текст и подготовка' }));
    expect(confirm).toHaveBeenCalledTimes(1);
    expect(saved().tabs).toHaveLength(1);
    confirm.mockReturnValue(true);
    fireEvent.click(screen.getByRole('button', { name: 'Закрыть Текст и подготовка' }));
    await waitFor(() => expect(saved().tabs).toHaveLength(0));
    expect(saved().activeTab).toBeNull();
  },
);
it('keeps shared speech/text context and uses accessible keyboard tab navigation', async () => {
  data.state.workspaces[0].tabs.push({ id: 'two', tool: 'speech', assetId: asset.id, draft: '{}' });
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  await setup();
  const first = screen.getByRole('tab', { name: /Текст и подготовка/ });
  const second = screen.getByRole('tab', { name: /Синтез речи/ });
  first.focus();
  fireEvent.keyDown(first, { key: 'ArrowRight' });
  await waitFor(() => expect(second.getAttribute('aria-selected')).toBe('true'));
  expect(document.activeElement).toBe(second);
  fireEvent.keyDown(second, { key: 'Home' });
  await waitFor(() => expect(first.getAttribute('aria-selected')).toBe('true'));
  fireEvent.keyDown(first, { key: 'End' });
  await waitFor(() => expect(second.getAttribute('aria-selected')).toBe('true'));
  expect(first.getAttribute('aria-controls')).toBe(second.getAttribute('aria-controls'));
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть Синтез речи' }));
  await waitFor(() => expect(saved().tabs).toHaveLength(1));
  expect(saved().activeTab).toBe('one');
  expect(confirm).not.toHaveBeenCalled();
});

it('persists pointer tab selection and preserves the accepted tab when saving fails until retry', async () => {
  const draft = JSON.stringify({ text: 'Accepted shared speech draft' });
  data.state.workspaces[0].tabs[0].draft = draft;
  data.state.workspaces[0].tabs.push({ id: 'two', tool: 'speech', assetId: asset.id, draft });
  await setup();
  const text = screen.getByRole('tab', { name: /Текст и подготовка/ });
  const speech = screen.getByRole('tab', { name: /Синтез речи/ });
  fireEvent.click(speech);
  await waitFor(() => expect(saved().activeTab).toBe('two'));
  const failedSave = vi.fn(async () => {
    throw new Error('Tab persistence unavailable');
  });
  data.overrides.save_library_cmd = failedSave;
  fireEvent.click(text);
  await waitFor(() => expect(failedSave).toHaveBeenCalledOnce());
  expect(saved().activeTab).toBe('two');
  expect(speech.getAttribute('aria-selected')).toBe('true');
  expect(text.getAttribute('aria-selected')).toBe('false');
  expect(saved().tabs.map((tab) => tab.draft)).toEqual([draft, draft]);
  delete data.overrides.save_library_cmd;
  fireEvent.click(text);
  await waitFor(() => expect(saved().activeTab).toBe('one'));
  expect(text.getAttribute('aria-selected')).toBe('true');
  expect(saved().tabs).toHaveLength(2);
  expect(saved().tabs.map((tab) => tab.draft)).toEqual([draft, draft]);
});
it('changes command context through material selection and keeps foreign/hidden materials out of the selector', async () => {
  data.state.assets.push(
    { ...asset, id: 'second', name: 'Second text' },
    { ...asset, id: 'hidden', name: 'Hidden text' },
    { ...asset, id: 'linked', name: 'Foreign linked' },
  );
  data.state.hiddenAssets = ['hidden'];
  data.state.links = [{ projectId: 'foreign', assetId: 'linked', folderId: null }];
  await setup();
  const selector = screen.getByRole('combobox', { name: 'Материал для инструмента' });
  expect(screen.queryByRole('option', { name: 'Hidden text' })).toBeNull();
  expect(screen.queryByRole('option', { name: 'Foreign linked' })).toBeNull();
  fireEvent.change(selector, { target: { value: 'second' } });
  fireEvent.click(screen.getByRole('menuitem', { name: 'Инструменты' }));
  fireEvent.click(screen.getByRole('menuitem', { name: 'Синтез речи' }));
  await waitFor(() =>
    expect(
      snapshot(screen.getByLabelText('Library snapshot')).workspaces.find(
        (item) => item.id === 'asset:second',
      )?.tabs[0]?.assetId,
    ).toBe('second'),
  );
  expect(saved().tabs).toHaveLength(1);
});
it('reports a rejected close and preserves the accepted tab', async () => {
  await setup();
  data.overrides.save_library_cmd = () => {
    throw new Error('Workspace offline');
  };
  fireEvent.click(screen.getByRole('button', { name: 'Закрыть Текст и подготовка' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Workspace offline');
  expect(saved().tabs).toHaveLength(1);
});
