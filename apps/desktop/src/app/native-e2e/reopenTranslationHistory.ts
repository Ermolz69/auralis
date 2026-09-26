import { listProjects } from '@/entities/project';
import {
  getHistoricalTranslationEditContext,
  getTranslationComparison,
  listProjectTranslations,
  listTranslationHistory,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';
import { mountTranslationReview } from './translationReviewScenarioView';
import { TranslationHistoryControls } from './translationHistoryControls';
import { LATER_SECOND_LINE } from './translationHistoryFixture';

export async function reopenTranslationHistory(scenarioId: string): Promise<boolean> {
  const project = (await listProjects()).find(
    (item) => item.title === `native-e2e-translation-complete:${scenarioId}`,
  );
  if (!project) return false;
  const links = await listProjectTranslations(project.id);
  if (links.length !== 1) throw new Error('Reopened history lost its project link');
  const history = await listTranslationHistory(project.id, links[0].translationId);
  if (
    history.entries.length !== 3 ||
    history.link.selectedResultId !== history.entries[1].resultId ||
    history.link.activeRunId !== null
  )
    throw new Error('Startup recovery overwrote the explicit historical choice');
  const resultId = history.entries[1].resultId;
  const page = await getTranslationComparison(project.id, links[0].translationId, resultId, 0, 2);
  if (page.segments.length !== 2 || page.segments[1].translatedLines[0] !== LATER_SECOND_LINE)
    throw new Error('Reopened selected result lost the later second-segment correction');
  const view = mountTranslationReview(project.id);
  const controls = new TranslationHistoryControls(view.container);
  try {
    await controls.wait(() => {
      const selector = view.container.querySelector<HTMLSelectElement>(
        '#translation-history-selection',
      );
      return Boolean(
        selector &&
        !selector.disabled &&
        selector.value === resultId &&
        controls.hasParagraph(LATER_SECOND_LINE) &&
        controls.hasParagraph(page.segments[0].translatedLines[0]),
      );
    });
    await controls.open('你好。');
    const context = await getHistoricalTranslationEditContext(
      project.id,
      links[0].translationId,
      resultId,
    );
    if (
      context.expectedHeadResultId !== history.entries[0].resultId ||
      context.baseRevision !== 2 ||
      context.headRevision !== 3 ||
      context.expectedLinkRevision !== history.link.revision
    )
      throw new Error('Reopened historical editor did not observe the newer run head');
    if ((await listProjectTranslations(project.id))[0].selectedResultId !== resultId)
      throw new Error('Opening a verified editor mutated selection');
    await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-history-reopened' });
    return true;
  } finally {
    view.dispose();
  }
}
