import { useState } from 'react';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useTabDraft } from './tabDraft';
import { asset, backend, LibraryHarness, snapshot } from '../../../../tests/studio';
const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => api);
let data: ReturnType<typeof backend>;
const tab = {
  id: 'draft-tab',
  assetId: asset.id,
  tool: 'text' as const,
  draft: '{"text":"Saved"}',
};
beforeEach(() => {
  vi.resetAllMocks();
  data = backend();
  data.state.workspaces = [
    { id: 'workspace', projectId: null, assetId: asset.id, activeTab: tab.id, tabs: [tab] },
  ];
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
});
afterEach(cleanup);
function Editor() {
  const { initial, change, error, retry } = useTabDraft(tab);
  return (
    <>
      <span>{initial.text}</span>
      <button onClick={() => change({ text: 'Edited' })}>Edit</button>
      {error && (
        <p role="alert">
          {error}
          <button onClick={() => void retry()}>Retry draft</button>
        </p>
      )}
    </>
  );
}
function Route() {
  const [open, setOpen] = useState(true);
  return (
    <>
      <button onClick={() => setOpen(!open)}>Toggle editor</button>
      {open && <Editor />}
    </>
  );
}
async function setup() {
  render(
    <LibraryHarness>
      <Route />
    </LibraryHarness>,
  );
  await waitFor(() =>
    expect(screen.getByLabelText('Library snapshot').getAttribute('data-loaded')).toBe('true'),
  );
}
const savedText = () =>
  JSON.parse(snapshot(screen.getByLabelText('Library snapshot')).workspaces[0].tabs[0].draft).text;
it('debounces draft persistence and avoids duplicate writes for the same edit', async () => {
  await setup();
  expect(screen.getByText('Saved')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  await waitFor(() => expect(savedText()).toBe('Edited'));
  expect(api.invoke.mock.calls.filter(([command]) => command === 'save_library_cmd')).toHaveLength(
    1,
  );
});
it('flushes an edit when its panel unmounts before the debounce deadline', async () => {
  await setup();
  fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  fireEvent.click(screen.getByRole('button', { name: 'Toggle editor' }));
  await waitFor(() => expect(savedText()).toBe('Edited'));
});
it('surfaces failed persistence without replacing the accepted saved draft', async () => {
  await setup();
  data.overrides.save_library_cmd = () => {
    throw new Error('Draft storage unavailable');
  };
  fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Draft storage unavailable');
  expect(savedText()).toBe('Saved');
  delete data.overrides.save_library_cmd;
  fireEvent.click(screen.getByRole('button', { name: 'Retry draft' }));
  await waitFor(() => expect(savedText()).toBe('Edited'));
  expect(screen.queryByRole('alert')).toBeNull();
});
