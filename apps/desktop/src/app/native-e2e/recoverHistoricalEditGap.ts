import { listProjects, renameProject } from '@/entities/project';
import {
  getTranslationComparison,
  getTranslationPublication,
  listProjectTranslations,
  listTranslationHistory,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';
import { mountTranslationReview } from './translationReviewScenarioView';
import { TranslationHistoryControls } from './translationHistoryControls';
import { BRANCH_FIRST_LINE } from './translationHistoryFixture';
import { retryHistoricalJournal } from './retryHistoricalJournal';

export async function recoverHistoricalEditGap(scenarioId: string): Promise<boolean> {
  const projects = await listProjects();
  const project = projects.find(
    (item) => item.title === `native-e2e-translation-running:${scenarioId}`,
  );
  if (!project) return false;
  const links = await listProjectTranslations(project.id);
  if (links.length !== 1) throw new Error('Historical gap lost its project link');
  const translationId = links[0].translationId;
  const view = mountTranslationReview(project.id);
  const controls = new TranslationHistoryControls(view.container);
  try {
    if (__NATIVE_E2E_TRANSLATION_HISTORY_JOURNAL_GAP__)
      await retryHistoricalJournal(controls, project.id, translationId);
    await controls.wait(
      async () => (await listTranslationHistory(project.id, translationId)).entries.length === 3,
    );
    const history = await listTranslationHistory(project.id, translationId);
    const branchId = history.entries[0].resultId;
    const modelId = history.entries[2].resultId;
    const outcome = await getTranslationPublication(project.id, translationId, branchId);
    if (
      outcome.publication.state !== 'ready' ||
      outcome.link.selectedResultId !== modelId ||
      outcome.link.activeRunId !== null ||
      outcome.link.revision !== 4
    )
      throw new Error('Historical gap recovery overwrote the newer explicit project choice');
    const original = await getTranslationComparison(project.id, translationId, modelId, 0, 2);
    await controls.wait(() => controls.hasParagraph(original.segments[0].translatedLines[0]));
    const selector = view.container.querySelector<HTMLSelectElement>(
      '#translation-history-selection',
    );
    if (selector?.value !== modelId)
      throw new Error('Reopened UI did not retain the model selection');
    await controls.preview(branchId, BRANCH_FIRST_LINE);
    if (!controls.hasParagraph(original.segments[1].translatedLines[0]))
      throw new Error('Recovered historical branch lost its original second cue');
    const link = (await listProjectTranslations(project.id))[0];
    if (link.selectedResultId !== modelId || !controls.button('Использовать эту версию'))
      throw new Error('Recovered branch preview changed attachment or is not selectable');
    await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-history-gap-recovered' });
    await renameProject(
      project.id,
      `native-e2e-translation-complete:${scenarioId}`,
      project.revision,
    );
    const main = projects.find((item) => item.title === `native-e2e-running:${scenarioId}`);
    if (!main) throw new Error('Historical gap lost its original native workflow project');
    const current = await invoke('get_project_cmd', { projectId: main.id });
    await renameProject(main.id, `native-e2e-complete:${scenarioId}`, current.revision);
    return true;
  } finally {
    view.dispose();
  }
}
