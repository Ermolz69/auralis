import { createProject, listProjects, renameProject } from '@/entities/project';
import { invoke } from '@/shared/api/tauri';

const TIMEOUT_MS = 30_000;
const POLL_MS = 50;

export async function runCasterScenario(runId: string, showProject: (projectId: string) => void) {
  const completed = `native-e2e-caster-complete:${runId}`;
  const previous = (await listProjects()).find((project) => project.title === completed);
  if (previous) {
    showProject(previous.id);
    const current = await invoke('get_manual_cast_cmd', { projectId: previous.id });
    if (!current?.revision.selections.narrator) throw new Error('Restart lost the assigned voice');
    await waitFor(
      () =>
        document.body.textContent?.includes(
          `Активная ревизия: ${current.revision_sha256.slice(0, 12)}`,
        ) ?? false,
    );
    const latest = await invoke('get_project_cmd', { projectId: previous.id });
    await renameProject(previous.id, `native-e2e-caster-reopened:${runId}`, latest.revision);
    return;
  }

  const project = await createProject(`native-e2e-caster-running:${runId}`);
  try {
    showProject(project.id);
    await clickButton('Открыть');
    await clickButton('Слушать');
    await waitFor(() => {
      const audio = document.querySelector('audio');
      return !!audio?.getAttribute('src') && audio.readyState >= HTMLMediaElement.HAVE_METADATA;
    });
    await clickButton('Назначить роли');
    await waitFor(() => document.body.textContent?.includes('Сохраните ревизию') ?? false);
    await clickButton('Сохранить cast');
    await waitFor(() => document.body.textContent?.includes('сохранена') ?? false);
    const current = await invoke('get_manual_cast_cmd', { projectId: project.id });
    if (!current?.revision.selections.narrator)
      throw new Error('Cast assignment was not published');
    const artifacts = await invoke('list_project_artifacts_cmd', { projectId: project.id });
    const managed = artifacts.filter(
      (artifact) => artifact.kind === 'manualCast' && artifact.state === 'ready',
    );
    if (managed.length !== 1 || managed[0].location.kind !== 'storageKey') {
      throw new Error('Manual cast is not a complete managed artifact');
    }
    const resolved = await invoke('resolve_artifact_path_cmd', { artifactId: managed[0].id });
    if (!resolved.endsWith(`${current.revision_sha256}.json`))
      throw new Error('Managed cast path differs from the active revision');
    const latest = await invoke('get_project_cmd', { projectId: project.id });
    await renameProject(project.id, completed, latest.revision);
  } catch (error) {
    const latest = await invoke('get_project_cmd', { projectId: project.id }).catch(() => null);
    if (latest)
      await renameProject(project.id, `native-e2e-caster-failed:${runId}`, latest.revision).catch(
        () => undefined,
      );
    throw error;
  }
}

async function clickButton(label: string) {
  const button = await waitFor(() =>
    [...document.querySelectorAll('button')].find(
      (candidate) => candidate.textContent?.trim() === label && !candidate.disabled,
    ),
  );
  button.click();
}

async function waitFor<T>(observe: () => T | undefined | null | false): Promise<T> {
  const deadline = Date.now() + TIMEOUT_MS;
  while (Date.now() < deadline) {
    const value = observe();
    if (value) return value;
    await new Promise((resolve) => setTimeout(resolve, POLL_MS));
  }
  throw new Error('Timed out waiting for the native Caster UI');
}
