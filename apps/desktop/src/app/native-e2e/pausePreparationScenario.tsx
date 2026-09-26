import { createRoot } from 'react-dom/client';
import { renameProject } from '@/entities/project';
import { getTranslationRunStatus, listProjectTranslations } from '@/entities/translation';
import { TranslationRunControl } from '@/features/translation-run-control';
import { invoke } from '@/shared/api/tauri';

const PREPARATION_TIMEOUT_MS = 180_000;
const ACKNOWLEDGEMENT_TIMEOUT_MS = 10_000;
const POLL_INTERVAL_MS = 25;

export async function pausePreparationScenario(
  scenarioId: string,
  projectId: string,
  translationId: string,
  runId: string,
): Promise<void> {
  const container = document.createElement('div');
  document.body.append(container);
  let root = createRoot(container);
  root.render(<TranslationRunControl projectId={projectId} />);
  try {
    (await waitForButton(container, 'Начать перевод')).click();
    await waitFor(container, () => container.textContent?.includes('Подготовка модели'));
    await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-preparation-started' });
    await waitForTitle(projectId, `native-e2e-preparation-pause-now:${scenarioId}`);
    if (!container.textContent?.includes('Подготовка модели')) {
      throw new Error('Model preparation finished before the observed pause boundary');
    }
    const clickedAt = performance.now();
    (await waitForButton(container, 'Пауза')).click();
    await waitFor(
      container,
      () =>
        !container.textContent?.includes('Подготовка модели') &&
        container.textContent?.includes('На паузе') &&
        !!findButton(container, 'Продолжить перевод'),
      ACKNOWLEDGEMENT_TIMEOUT_MS,
    );
    const acknowledgementMs = Math.ceil(performance.now() - clickedAt);
    const status = await getTranslationRunStatus(projectId, translationId);
    const link = (await listProjectTranslations(projectId)).find(
      (item) => item.translationId === translationId,
    );
    if (
      status.phase !== 'paused' ||
      status.pauseRequested ||
      status.savedBlocks !== 0 ||
      link?.activeRunId !== runId ||
      link.selectedResultId ||
      link.outputArtifactId
    ) {
      throw new Error('Preparation pause changed the run or exposed partial translation');
    }
    const project = await invoke('get_project_cmd', { projectId });
    await renameProject(
      projectId,
      `native-e2e-preparation-paused:${scenarioId}:${acknowledgementMs}`,
      project.revision,
    );
    await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-preparation-paused' });
    await waitForTitle(projectId, `native-e2e-preparation-resume:${scenarioId}`);
    (await waitForButton(container, 'Продолжить перевод')).click();
    await waitFor(container, () => container.textContent?.includes('Подготовка модели'));
    await waitForWorkerPreflight(projectId, translationId);
    root.unmount();
    root = createRoot(container);
    root.render(<TranslationRunControl projectId={projectId} />);
    await waitFor(container, () => container.textContent?.includes('Подготовка модели'));
    if (!findButton(container, 'Пауза') || findButton(container, 'Продолжить перевод')) {
      throw new Error('Remounted preflight controls lost pause or exposed duplicate resume');
    }
    await invoke('native_e2e_checkpoint_cmd', {
      checkpoint: 'translation-worker-preflight-visible',
    });
    await waitFor(container, () => !container.textContent?.includes('Подготовка модели'));
    await waitForResumedAttempt(container, projectId, translationId);
  } finally {
    root.unmount();
    container.remove();
  }
}

async function waitForButton(container: HTMLElement, label: string): Promise<HTMLButtonElement> {
  await waitFor(container, () => !!findButton(container, label));
  return findButton(container, label)!;
}

function findButton(container: HTMLElement, label: string): HTMLButtonElement | undefined {
  return [...container.querySelectorAll('button')].find(
    (button) => button.textContent?.trim() === label && !button.disabled,
  );
}

async function waitFor(
  container: HTMLElement,
  condition: () => unknown,
  timeoutMs = PREPARATION_TIMEOUT_MS,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const alert = container.querySelector('[role="alert"]');
    if (alert) throw new Error(`Native preparation control failed: ${alert.textContent?.trim()}`);
    if (condition()) return;
    await delay();
  }
  throw new Error('Timed out waiting for the native preparation control');
}

async function waitForTitle(projectId: string, title: string): Promise<void> {
  const deadline = Date.now() + PREPARATION_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if ((await invoke('get_project_cmd', { projectId })).title === title) return;
    await delay();
  }
  throw new Error('Native harness did not release the preparation boundary');
}

async function waitForResumedAttempt(
  container: HTMLElement,
  projectId: string,
  translationId: string,
): Promise<void> {
  const deadline = Date.now() + PREPARATION_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const alert = container.querySelector('[role="alert"]');
    if (alert) throw new Error(`Native resume control failed: ${alert.textContent?.trim()}`);
    const status = await getTranslationRunStatus(projectId, translationId);
    if (status.phase === 'running' || status.phase === 'validated') return;
    if (status.phase !== 'paused') {
      throw new Error(`Fresh preparation resume ended in ${status.phase}`);
    }
    await delay();
  }
  throw new Error('Fresh preparation resume did not reach its guarded attempt');
}

async function waitForWorkerPreflight(projectId: string, translationId: string): Promise<void> {
  const deadline = Date.now() + PREPARATION_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const status = await getTranslationRunStatus(projectId, translationId);
    if (status.activeHostJobId && status.phase === 'paused' && !status.pauseRequested) return;
    if (status.phase !== 'paused') {
      throw new Error('Native resume missed the accepted worker-preflight boundary');
    }
    await delay();
  }
  throw new Error('Native resume did not expose its active preflight host job');
}

function delay(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
}
