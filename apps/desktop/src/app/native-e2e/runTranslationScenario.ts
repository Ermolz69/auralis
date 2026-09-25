import { createProject, listProjects, renameProject } from '@/entities/project';
import {
  beginTranslationRun,
  getTranslationComparison,
  importTranslationSource,
  listOriginalSubtitles,
  listProjectTranslations,
  startTranslationRun,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';

const ARTIFACT_TIMEOUT_MS = 20_000;
const PUBLICATION_TIMEOUT_MS = 60_000;
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

  const publication = await startTranslationRun(project.id, translationId);
  await finishTranslationScenario(runId, project.id, translationId, translationRunId, publication);
}

export async function resumeTranslationScenario(runId: string): Promise<boolean> {
  const project = (await listProjects()).find(
    (item) => item.title === `native-e2e-translation-running:${runId}`,
  );
  if (!project) return false;
  const links = await listProjectTranslations(project.id);
  if (links.length !== 1 || !links[0].activeRunId) {
    throw new Error('Native translation crash recovery lost the active project run');
  }
  const link = links[0];
  const activeRunId = link.activeRunId;
  if (!activeRunId) throw new Error('Native translation crash recovery lost its run ID');
  const publication = await startTranslationRun(project.id, link.translationId);
  await finishTranslationScenario(runId, project.id, link.translationId, activeRunId, publication);
  return true;
}

async function finishTranslationScenario(
  runId: string,
  projectId: string,
  translationId: string,
  translationRunId: string,
  publication: Awaited<ReturnType<typeof startTranslationRun>>,
): Promise<void> {
  if (publication.runId !== translationRunId || !publication.needsReview) {
    throw new Error('Native translation returned the wrong run or review state');
  }
  const ready = await waitForSelection(projectId, translationId, publication.resultId);
  if (ready.outputArtifactId !== publication.artifactId) {
    throw new Error('Native translation selected a different output artifact');
  }
  const comparison = await getTranslationComparison(
    projectId,
    translationId,
    publication.resultId,
    0,
    10,
  );
  const expectedSegments = __NATIVE_E2E_TRANSLATION_CRASH__ ? 2 : 1;
  if (
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

async function waitForSelection(projectId: string, translationId: string, resultId: string) {
  const deadline = Date.now() + PUBLICATION_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const link = (await listProjectTranslations(projectId)).find(
      (item) => item.translationId === translationId,
    );
    if (link?.selectedResultId === resultId && link.outputArtifactId && !link.activeRunId) {
      return link;
    }
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Native translation artifact did not become the selected result');
}

function delay(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
