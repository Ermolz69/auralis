import { useEffect } from 'react';
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useLibrary } from '@/entities/library';
import type { LibraryAsset, LibraryTab, LibraryWorkspace } from '@/shared/api/contracts/library';
import { SpeechTool } from './SpeechTool';
import {
  asset,
  artifact,
  backend,
  deferred,
  LibraryHarness,
  project,
  snapshot,
} from '../../../../../tests/studio';
const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => api);
let data: ReturnType<typeof backend>;
const tab: LibraryTab = { id: 'text-tab', tool: 'text', assetId: asset.id, draft: '{}' };
const workspace: LibraryWorkspace = {
  id: 'workspace',
  projectId: null,
  assetId: asset.id,
  activeTab: tab.id,
  tabs: [tab],
};
function Activate() {
  const { loading, activateWorkspace } = useLibrary();
  useEffect(() => {
    if (!loading) activateWorkspace(workspace.id);
  }, [loading]);
  return null;
}
const view = (selected: LibraryAsset = asset, selectedTab = tab) => (
  <LibraryHarness>
    <Activate />
    <SpeechTool projectId={project.id} asset={selected} tab={selectedTab} workspace={workspace} />
  </LibraryHarness>
);
beforeEach(() => {
  vi.resetAllMocks();
  data = backend();
  data.state.workspaces = [structuredClone(workspace)];
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
});
afterEach(cleanup);
it('binds a speech plan to the reviewed digest and refuses reuse after editing', async () => {
  const source = {
    kind: 'text',
    source_id: null,
    revision: 'version-one',
    sha256: 'a'.repeat(64),
    text: asset.text,
    cues: [],
  };
  data.overrides.review_speech_text_cmd = () => ({
    decision_id: 'review-one',
    project_id: project.id,
    source,
  });
  data.overrides.prepare_speech_plan_cmd = () => ({
    run_id: 'planned',
    profile_id: 'native-plan-voxcpm2',
    utterance_count: 1,
    chunk_count: 1,
    job: null,
    result: null,
  });
  render(view(asset, { ...tab, tool: 'speech' }));
  const input = await screen.findByRole('textbox');
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  await screen.findByText('Снимок проверенного текста сохранён.');
  fireEvent.click(screen.getByRole('button', { name: 'Подготовить озвучку' }));
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith('prepare_speech_plan_cmd', {
      projectId: project.id,
      assetId: asset.id,
      request: {
        decision_id: 'review-one',
        expected_source_sha256: source.sha256,
        profile_id: 'native-plan-voxcpm2',
        voice_id: 'fixture-voice',
        reference_id: 'neutral',
        assignments: [],
      },
    }),
  );
  fireEvent.change(input, { target: { value: 'A new version' } });
  expect(screen.getByRole('button', { name: 'Подготовить озвучку' })).toHaveProperty(
    'disabled',
    true,
  );
  expect(screen.queryByText('Снимок проверенного текста сохранён.')).toBeNull();
});
it('rejects a review returned for another project', async () => {
  data.overrides.review_speech_text_cmd = () => ({
    decision_id: 'foreign',
    project_id: 'other',
    source: {},
  });
  render(view(asset, { ...tab, tool: 'speech' }));
  await screen.findByRole('textbox');
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Текст проверен' })).toHaveProperty(
      'disabled',
      false,
    ),
  );
  expect(screen.queryByText('Снимок проверенного текста сохранён.')).toBeNull();
  expect(screen.getByRole('button', { name: 'Подготовить озвучку' })).toHaveProperty(
    'disabled',
    true,
  );
});
it('admits a repeated save once while the version owner is being created', async () => {
  const pending = deferred<unknown>();
  data.overrides.create_project_cmd = () => pending.promise;
  render(view());
  await screen.findByRole('textbox');
  const save = screen.getByRole('button', { name: 'Сохранить текст новой версией' });
  act(() => {
    fireEvent.click(save);
    fireEvent.click(save);
  });
  expect(
    api.invoke.mock.calls.filter(([command]) => command === 'create_project_cmd'),
  ).toHaveLength(1);
  await act(async () => pending.reject(new Error('Owner creation failed')));
  expect((await screen.findByRole('alert')).textContent).toContain('Owner creation failed');
});
it('restores a saved draft, persists edits and saves an independent version with material lineage', async () => {
  const selectedTab = { ...tab, draft: JSON.stringify({ text: 'Saved draft', mode: 'text' }) };
  data.state.workspaces[0].tabs = [selectedTab];
  render(view(asset, selectedTab));
  const input = await screen.findByRole('textbox', { name: 'Русский текст для озвучки' });
  expect(input).toHaveProperty('value', 'Saved draft');
  fireEvent.change(input, { target: { value: 'Edited draft' } });
  await waitFor(() =>
    expect(
      JSON.parse(snapshot(screen.getByLabelText('Library snapshot')).workspaces[0].tabs[0].draft)
        .text,
    ).toBe('Edited draft'),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить текст новой версией' }));
  await waitFor(() =>
    expect(snapshot(screen.getByLabelText('Library snapshot')).assets).toHaveLength(2),
  );
  const items = snapshot(screen.getByLabelText('Library snapshot')).assets;
  expect(items[0].text).toBe(asset.text);
  expect(items[1]).toMatchObject({
    text: 'Edited draft',
    parentId: asset.id,
    name: `${asset.name} · версия`,
  });
  expect(items[1].ownerProjectId).not.toBe(asset.ownerProjectId);
});
it('reads a managed text artifact and reports failed version publication without losing the edit', async () => {
  data.overrides.read_library_text_cmd = () => 'Stored artifact text';
  render(view({ ...asset, text: null, artifactId: 'stored-text' }));
  const input = await screen.findByRole('textbox', { name: 'Русский текст для озвучки' });
  expect(input).toHaveProperty('value', 'Stored artifact text');
  expect(api.invoke).toHaveBeenCalledWith('read_library_text_cmd', {
    path: 'C:/fixture/stored-text.wav',
  });
  fireEvent.change(input, { target: { value: 'Keep this edit' } });
  data.overrides.create_project_cmd = () => {
    throw new Error('Version storage offline');
  };
  fireEvent.click(screen.getByRole('button', { name: 'Сохранить текст новой версией' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Version storage offline');
  expect(input).toHaveProperty('value', 'Keep this edit');
  expect(screen.getByRole('button', { name: 'Сохранить текст новой версией' })).toHaveProperty(
    'disabled',
    false,
  );
});
it.each(['srt', 'vtt'])(
  'preserves %s subtitle identity when there is no selected translation output',
  async (format) => {
    data.artifacts = [
      artifact('subtitle-source', {
        kind: 'originalSubtitle',
        location: { kind: 'storageKey', value: `source.${format}` },
      }),
    ];
    render(
      view(
        { ...asset, kind: 'subtitle', artifactId: 'subtitle-source', format },
        { ...tab, tool: 'speech' },
      ),
    );
    await waitFor(() =>
      expect(screen.getByLabelText('Источник субтитров')).toHaveProperty(
        'value',
        'artifact:subtitle-source',
      ),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
    await waitFor(() =>
      expect(api.invoke).toHaveBeenCalledWith(
        'review_speech_text_cmd',
        expect.objectContaining({
          projectId: project.id,
          source: expect.objectContaining({ artifact_id: 'subtitle-source', format }),
        }),
      ),
    );
  },
);
it('uses only this project selected translation output as the speech source', async () => {
  const selected = {
    ...asset,
    kind: 'subtitle' as const,
    artifactId: 'translated-output',
    format: 'srt',
  };
  data.translations = [
    {
      translationId: 'translation-one',
      projectId: project.id,
      selectedResultId: 'result-one',
      outputArtifactId: selected.artifactId,
    } as (typeof data.translations)[number],
  ];
  render(view(selected, { ...tab, tool: 'speech' }));
  await waitFor(() =>
    expect(screen.getByLabelText('Источник субтитров')).toHaveProperty(
      'value',
      'translation:translation-one',
    ),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Использовать одобренный перевод' }));
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith('review_speech_text_cmd', {
      projectId: project.id,
      source: {
        kind: 'translation',
        text: null,
        translation_id: 'translation-one',
        artifact_id: null,
        format: null,
      },
      confirmed: true,
    }),
  );
});
it.each(['foreign-project', 'unselected-result'])(
  'rejects a translation with %s lineage',
  async (invalid) => {
    const selected = {
      ...asset,
      kind: 'subtitle' as const,
      artifactId: 'translated-output',
      format: 'vtt',
    };
    data.translations = [
      {
        translationId: 'foreign',
        projectId: invalid === 'foreign-project' ? 'other' : project.id,
        selectedResultId: invalid === 'unselected-result' ? null : 'result',
        outputArtifactId: selected.artifactId,
      } as (typeof data.translations)[number],
    ];
    render(view(selected, { ...tab, tool: 'speech' }));
    await screen.findByRole('button', { name: 'Текст проверен' });
    expect(screen.queryByRole('button', { name: 'Использовать одобренный перевод' })).toBeNull();
  },
);
it('does not retain the previous source error while a new material is resolving', async () => {
  data.overrides.read_library_text_cmd = () => {
    throw new Error('Old source unreadable');
  };
  const rendered = render(view({ ...asset, text: null, artifactId: 'broken' }));
  await screen.findByRole('alert');
  const pending = deferred<string>();
  data.overrides.read_library_text_cmd = () => pending.promise;
  rendered.rerender(view({ ...asset, id: 'new-material', text: null, artifactId: 'new-text' }));
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.queryByRole('textbox')).toBeNull();
  await act(async () => pending.resolve('New source'));
  expect(await screen.findByRole('textbox')).toHaveProperty('value', 'New source');
});
it('ignores a late old text reader after switching material', async () => {
  const pending = deferred<string>();
  data.overrides.read_library_text_cmd = () => pending.promise;
  const rendered = render(view({ ...asset, text: null, artifactId: 'slow' }));
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith('read_library_text_cmd', expect.anything()),
  );
  rendered.rerender(view({ ...asset, id: 'new-material', text: 'New material text' }));
  const input = await screen.findByRole('textbox');
  expect(input).toHaveProperty('value', 'New material text');
  await act(async () => pending.resolve('Late old text'));
  expect(input).toHaveProperty('value', 'New material text');
});
it('invalidates approval on editing and does not restore a late review', async () => {
  const pending = deferred<unknown>();
  data.overrides.review_speech_text_cmd = () => pending.promise;
  render(view(asset, { ...tab, tool: 'speech' }));
  const input = await screen.findByRole('textbox');
  fireEvent.click(screen.getByRole('button', { name: 'Текст проверен' }));
  fireEvent.change(input, { target: { value: 'Changed during approval' } });
  await act(async () =>
    pending.resolve({
      decision_id: 'old-review',
      project_id: project.id,
      source: {
        kind: 'text',
        source_id: null,
        text: asset.text,
        cues: [],
        revision: 'one',
        sha256: 'a'.repeat(64),
      },
    }),
  );
  expect(screen.queryByText('Снимок проверенного текста сохранён.')).toBeNull();
  expect(screen.getByRole('button', { name: 'Подготовить озвучку' })).toHaveProperty(
    'disabled',
    true,
  );
});
