import { createProject, deleteProject, listProjects, renameProject } from '@/entities/project';
import {
  beginTranslationRun,
  getTranslationComparison,
  getTranslationRunStatus,
  importTranslationSource,
  listOriginalSubtitles,
  listProjectTranslations,
  startTranslationRun,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';

const ARTIFACT_TIMEOUT_MS = 20_000;
const PUBLICATION_TIMEOUT_MS = 180_000;
const ACTIVE_ATTEMPT_TIMEOUT_MS = 180_000;
const POLL_INTERVAL_MS = 100;

export async function runTranslationScenario(runId: string, sourcePath: string): Promise<void> {
  const project = await createProject(`native-e2e-translation-running:${runId}`);
  const source = await importTranslationSource(project.id, sourcePath, 'srt');
  await waitForOriginal(project.id, source.id);
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-source-ready' });

  const translationId = crypto.randomUUID();
  const translationRunId = crypto.randomUUID();
  const link = await beginTranslationRun({
    projectId: project.id,
    sourceArtifactId: source.id,
    format: 'srt',
    sourceLanguage: 'zh',
    translationId,
    runId: translationRunId,
  });
  if (link.activeRunId !== translationRunId || link.selectedResultId !== null) {
    throw new Error('Native translation did not retain its active project run');
  }
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-run-ready' });

  const start = await startTranslationRun(project.id, translationId);
  if (__NATIVE_E2E_TRANSLATION_DELETE__) {
    await waitForRunningAttempt(project.id, translationId);
    await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-delete-requested' });
    await deleteProject(project.id);
    await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-project-deleted' });
    return;
  }
  await finishTranslationScenario(runId, project.id, translationId, translationRunId, start);
}

export async function resumeTranslationScenario(runId: string): Promise<boolean> {
  const project = (await listProjects()).find(
    (item) => item.title === `native-e2e-translation-running:${runId}`,
  );
  if (!project) return false;
  const links = await listProjectTranslations(project.id);
  if (links.length !== 1) {
    throw new Error('Native translation crash recovery lost the project link');
  }
  const link = links[0];
  let translationRunId: string;
  let start: Awaited<ReturnType<typeof startTranslationRun>> | null = null;
  if (link.activeRunId) {
    translationRunId = link.activeRunId;
    const status = await getTranslationRunStatus(project.id, link.translationId);
    if (status.phase !== 'validated') {
      start = await startTranslationRun(project.id, link.translationId);
    }
  } else if (link.selectedResultId) {
    const page = await getTranslationComparison(
      project.id,
      link.translationId,
      link.selectedResultId,
      0,
      1,
    );
    translationRunId = page.runId;
  } else {
    throw new Error('Native translation crash recovery lost its run and result IDs');
  }
  await finishTranslationScenario(runId, project.id, link.translationId, translationRunId, start);
  return true;
}

async function finishTranslationScenario(
  runId: string,
  projectId: string,
  translationId: string,
  translationRunId: string,
  start: Awaited<ReturnType<typeof startTranslationRun>> | null,
): Promise<void> {
  if (start && (start.runId !== translationRunId || start.state !== 'accepted')) {
    throw new Error('Native translation returned the wrong admitted run');
  }
  const ready = await waitForSelection(projectId, translationId);
  if (!ready.selectedResultId || !ready.outputArtifactId) throw new Error('Native translation selection is incomplete');
  const comparison = await getTranslationComparison(
    projectId,
    translationId,
    ready.selectedResultId,
    0,
    10,
  );
  const expectedSegments = __NATIVE_E2E_TRANSLATION_CRASH__ ? 2 : 1;
  if (
    comparison.runId !== translationRunId ||
    !comparison.needsReview ||
    comparison.totalSegments !== expectedSegments ||
    comparison.segments.length !== expectedSegments ||
    comparison.segments[0].sourceLines[0] !== '你好。' ||
    comparison.segments.some((segment) => !segment.translatedLines[0]?.trim())
  ) {
    throw new Error('Native translation comparison lost its source or result mapping');
  }
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-result-ready' });
  const latest = await invoke('get_project_cmd', { projectId });
  await renameProject(projectId, `native-e2e-translation-complete:${runId}`, latest.revision);
}

async function waitForOriginal(projectId: string, artifactId: string): Promise<void> {
  const deadline = Date.now() + ARTIFACT_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const original = (await listOriginalSubtitles(projectId)).find((item) => item.id === artifactId);
    if (original?.state === 'ready') return;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Native original subtitle did not become ready');
}

async function waitForRunningAttempt(projectId: string, translationId: string): Promise<void> {
  const deadline = Date.now() + ACTIVE_ATTEMPT_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const status = await getTranslationRunStatus(projectId, translationId);
    if (status.phase === 'running' && status.savedBlocks < status.totalBlocks) return;
    if (status.phase === 'validated') {
      throw new Error('Translation finished before the project-deletion boundary');
    }
    if (status.phase === 'failed') throw new Error('Translation failed before the project-deletion boundary');
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Timed out waiting for a running translation before deletion');
}

async function waitForSelection(projectId: string, translationId: string) {
  const deadline = Date.now() + PUBLICATION_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const link = (await listProjectTranslations(projectId)).find(
      (item) => item.translationId === translationId,
    );
    if (link?.selectedResultId && link.outputArtifactId && !link.activeRunId) {
      return link;
    }
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Native translation artifact did not become the selected result');
}

function delay(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
