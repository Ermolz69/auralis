import { renameProject } from '@/entities/project';
import {
  getTranslationRunStatus,
  listProjectTranslations,
  pauseTranslationRun,
  startTranslationRun,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';

const CHECKPOINT_TIMEOUT_MS = 180_000;
const PAUSE_TIMEOUT_MS = 10_000;
const RESUME_GATE_TIMEOUT_MS = 30_000;
const POLL_INTERVAL_MS = 25;

export async function pauseTranslationScenario(
  scenarioId: string,
  projectId: string,
  translationId: string,
  runId: string,
  firstJobId: string,
): Promise<Awaited<ReturnType<typeof startTranslationRun>>> {
  await waitForFirstCheckpoint(projectId, translationId);
  await pauseTranslationRun(projectId, translationId);
  const deadline = Date.now() + PAUSE_TIMEOUT_MS;
  let acknowledged = false;
  while (Date.now() < deadline) {
    const status = await getTranslationRunStatus(projectId, translationId);
    if (status.phase === 'paused' && !status.pauseRequested) {
      if (status.savedBlocks !== 1) throw new Error('Native pause lost a saved checkpoint');
      acknowledged = true;
      break;
    }
    if (status.phase === 'failed' || status.phase === 'validated') {
      throw new Error(`Native pause ended in ${status.phase}`);
    }
    await delay();
  }
  if (!acknowledged) throw new Error('Native pause did not acknowledge the request');
  const link = (await listProjectTranslations(projectId)).find(
    (item) => item.translationId === translationId,
  );
  if (link?.activeRunId !== runId || link.selectedResultId || link.outputArtifactId) {
    throw new Error('Native pause changed the active run or exposed partial output');
  }
  const project = await invoke('get_project_cmd', { projectId });
  await renameProject(projectId, `native-e2e-translation-paused:${scenarioId}`, project.revision);
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-pause-acknowledged' });
  const resumeDeadline = Date.now() + RESUME_GATE_TIMEOUT_MS;
  while (Date.now() < resumeDeadline) {
    const current = await invoke('get_project_cmd', { projectId });
    if (current.title === `native-e2e-translation-resume:${scenarioId}`) {
      const resumed = await startTranslationRun(projectId, translationId);
      if (resumed.runId !== runId || resumed.jobId === firstJobId) {
        throw new Error('Native resume did not create a new host attempt on the same run');
      }
      return resumed;
    }
    await delay();
  }
  throw new Error('Native harness did not verify and release the paused workflow');
}

async function waitForFirstCheckpoint(projectId: string, translationId: string): Promise<void> {
  const deadline = Date.now() + CHECKPOINT_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const status = await getTranslationRunStatus(projectId, translationId);
    if (status.phase === 'running' && status.savedBlocks === 1 && status.totalBlocks === 2) return;
    if (status.phase === 'failed' || status.phase === 'validated') {
      throw new Error(`Native translation reached ${status.phase} before pause`);
    }
    await delay();
  }
  throw new Error('Native translation did not reach the first saved checkpoint');
}

function delay(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
}
