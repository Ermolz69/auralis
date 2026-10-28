import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { TranslateTool } from './TranslateTool';
import {
  artifact,
  asset,
  backend,
  project,
  ProjectHarness,
  translationLink,
} from '../../../../../tests/studio';
const api = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock('@/shared/api/tauri', () => api);
let data: ReturnType<typeof backend>;
beforeEach(() => {
  vi.resetAllMocks();
  data = backend();
  api.invoke.mockImplementation((command, args) => data.handle(command, args));
  api.listen.mockResolvedValue(() => {});
  data.overrides.get_translation_package_status_cmd = () => ({
    schemaVersion: 1,
    releaseId: 'fixture',
    backend: 'cpu',
    supported: false,
    installed: false,
    selected: false,
  });
  data.overrides.get_project_translation_snapshot_cmd = () => ({
    projectId: project.id,
    entries: [],
  });
});
afterEach(cleanup);
it('keeps source import and run controls scoped to the selected subtitle artifact', async () => {
  data.artifacts = ['selected', 'foreign'].map((id) =>
    artifact(id, {
      kind: 'originalSubtitle',
      location: { kind: 'storageKey', value: `${id}.srt` },
    }),
  );
  data.overrides.begin_translation_run_cmd = ({ input }) =>
    translationLink({
      translationId: input.translationId,
      projectId: input.projectId,
      sourceArtifactId: input.sourceArtifactId,
      activeRunId: input.runId,
      selectedResultId: null,
      outputArtifactId: null,
    });
  render(
    <ProjectHarness>
      <TranslateTool asset={{ ...asset, kind: 'subtitle', artifactId: 'selected' }} />
    </ProjectHarness>,
  );
  const section = await screen.findByRole('region', { name: /Исходные субтитры/ });
  const prepare = await within(section).findByRole('button', { name: /Подготовить перевод/ });
  expect(within(section).getAllByRole('button', { name: /Подготовить перевод/ })).toHaveLength(1);
  fireEvent.click(prepare);
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith('begin_translation_run_cmd', {
      input: expect.objectContaining({
        projectId: project.id,
        sourceArtifactId: 'selected',
        format: 'srt',
      }),
    }),
  );
});
it('surfaces import boundary errors without loading a model', async () => {
  data.overrides.list_project_artifacts_cmd = () => {
    throw new Error('Sources offline');
  };
  render(
    <ProjectHarness>
      <TranslateTool asset={{ ...asset, kind: 'subtitle', artifactId: 'selected' }} />
    </ProjectHarness>,
  );
  expect(
    (await screen.findAllByRole('alert')).some((item) =>
      item.textContent?.includes('Sources offline'),
    ),
  ).toBe(true);
  expect(
    api.invoke.mock.calls.some(([command]) =>
      /install_translation|start_translation/.test(command),
    ),
  ).toBe(false);
});
it('waits for a project selection', async () => {
  render(
    <ProjectHarness selected={null}>
      <TranslateTool />
    </ProjectHarness>,
  );
  expect(screen.getByText('Загружаем контекст перевода…')).toBeTruthy();
  expect(api.invoke).not.toHaveBeenCalledWith(
    'get_translation_package_status_cmd',
    expect.anything(),
  );
});
