import { listProjects } from '@/entities/project';
import {
  getTranslationComparison,
  listProjectTranslations,
  listTranslationHistory,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';
import { mountTranslationReview } from './translationReviewScenarioView';

const TIMEOUT_MS = 20_000;
const POLL_INTERVAL_MS = 25;

export async function reopenTranslationHistory(scenarioId: string): Promise<boolean> {
  const project = (await listProjects()).find(
    (item) => item.title === `native-e2e-translation-complete:${scenarioId}`,
  );
  if (!project) return false;
  const links = await listProjectTranslations(project.id);
  if (links.length !== 1) throw new Error('Reopened history lost its project link');
  const history = await listTranslationHistory(project.id, links[0].translationId);
  if (
    history.entries.length !== 2 ||
    history.link.selectedResultId !== history.entries[1].resultId ||
    history.link.activeRunId !== null
  )
    throw new Error('Startup recovery overwrote the explicit historical choice');
  const page = await getTranslationComparison(
    project.id,
    links[0].translationId,
    history.entries[1].resultId,
    0,
    1,
  );
  const view = mountTranslationReview(project.id);
  try {
    const deadline = Date.now() + TIMEOUT_MS;
    while (Date.now() < deadline) {
      const selector = view.container.querySelector<HTMLSelectElement>(
        '#translation-history-selection',
      );
      if (
        selector &&
        !selector.disabled &&
        selector.value === history.entries[1].resultId &&
        Array.from(view.container.querySelectorAll('p')).some(
          (item) => item.textContent === page.segments[0].translatedLines[0],
        )
      ) {
        if (
          Array.from(view.container.querySelectorAll('button')).some(
            (item) => item.textContent?.trim() === 'Исправить перевод',
          )
        )
          throw new Error('Reopened historical version offers a stale edit');
        await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-history-reopened' });
        return true;
      }
      await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    }
    throw new Error('Reopened history did not render the selected original result');
  } finally {
    view.dispose();
  }
}
