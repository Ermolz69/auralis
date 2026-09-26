import { createRoot } from 'react-dom/client';
import { subscribeJobEvents } from '@/entities/job';
import { TranslationRunControl } from '@/features/translation-run-control';
import type { JobEvent } from '@/shared/api/contracts';
import { invoke } from '@/shared/api/tauri';

const SCENARIO_TIMEOUT_MS = 540_000;
const DOM_CHECK_INTERVAL_MS = 25;

export async function translationEventsScenario(projectId: string): Promise<void> {
  const container = document.createElement('div');
  document.body.append(container);
  const root = createRoot(container);
  const events: JobEvent[] = [];
  let readyNotifications = 0;
  const unlisten = await subscribeJobEvents((event) => {
    if (event.job.kind === 'translation' && event.job.projectId === projectId) events.push(event);
  });
  const deadline = Date.now() + SCENARIO_TIMEOUT_MS;
  const waitFor = async (predicate: () => boolean) => {
    while (Date.now() < deadline) {
      if (predicate()) return;
      await new Promise((resolve) => setTimeout(resolve, DOM_CHECK_INTERVAL_MS));
    }
    throw new Error('Timed out observing translation events and panel');
  };
  try {
    root.render(
      <TranslationRunControl
        projectId={projectId}
        onReadyResultsChanged={() => {
          readyNotifications++;
        }}
      />,
    );
    await waitFor(() => Boolean(findStartButton(container)));
    await checkpoint('translation-events-listener-ready');
    findStartButton(container)!.click();
    await waitFor(() => events.some((event) => event.kind === 'completed'));
    const created = events.find((event) => event.kind === 'created');
    const started = events.find((event) => event.kind === 'started');
    const saved = events.find(
      (event) => event.kind === 'progressed' && event.job.progress.processedItems === 1,
    );
    const completed = events.find((event) => event.kind === 'completed');
    if (!created || !started || !saved || !completed)
      throw new Error('Translation lifecycle events are incomplete');
    if (events.some((event) => event.job.id !== created.job.id || event.job.stage !== null)) {
      throw new Error('Translation events changed job identity or acquired a dubbing stage');
    }
    if (
      created.job.status !== 'pending' ||
      started.job.status !== 'running' ||
      saved.job.status !== 'running' ||
      saved.job.progress.totalItems !== 1 ||
      saved.job.progress.percent >= 100 ||
      completed.job.status !== 'completed' ||
      completed.job.progress.percent !== 100
    ) {
      throw new Error('Translation events lost committed progress or terminal semantics');
    }
    if (
      events.some(
        (event, index) => index > 0 && event.job.revision <= events[index - 1].job.revision,
      )
    ) {
      throw new Error('Translation event revisions are not increasing');
    }
    await checkpoint('translation-events-committed-progress');
    await waitFor(
      () =>
        readyNotifications === 1 &&
        Boolean(container.textContent?.includes('Результат прикреплён')),
    );
    await checkpoint('translation-events-panel-ready');
  } finally {
    unlisten();
    root.unmount();
    container.remove();
  }
}

function findStartButton(container: HTMLElement): HTMLButtonElement | undefined {
  return Array.from(container.querySelectorAll('button')).find(
    (button) => button.textContent?.trim() === 'Начать перевод',
  );
}

async function checkpoint(value: string): Promise<void> {
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: value });
}
