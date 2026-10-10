import {
  getTranslationComparison,
  getTranslationPublication,
  listProjectTranslations,
  listTranslationHistory,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';
import { mountTranslationReview } from './translationReviewScenarioView';
import { TranslationHistoryControls } from './translationHistoryControls';
import { BRANCH_FIRST_LINE, LATER_SECOND_LINE } from './translationHistoryFixture';
import { exportHistoryResult } from './translationHistoryExport';
import { chooseDuringHistoricalGap } from './chooseDuringHistoricalGap';

export async function translationHistoryScenario(
  projectId: string,
  translationId: string,
  originalResultId: string,
): Promise<void> {
  const original = await getTranslationComparison(projectId, translationId, originalResultId, 0, 2);
  if (original.segments.length !== 2 || original.segments[1].sourceLines[0] !== '再见。')
    throw new Error('Historical branch probe requires two mapped model cues');
  const originalFirst = original.segments[0].translatedLines[0];
  const originalSecond = original.segments[1].translatedLines[0];
  if (originalFirst === BRANCH_FIRST_LINE || originalSecond === LATER_SECOND_LINE)
    throw new Error('Authored manual lines must differ from the model output');
  const view = mountTranslationReview(projectId);
  const controls = new TranslationHistoryControls(view.container);
  const selected = async () =>
    (await listProjectTranslations(projectId)).find((link) => link.translationId === translationId);
  const waitNewResult = async (previousId: string): Promise<string> => {
    let resultId: string | null = null;
    await controls.wait(async () => {
      const link = await selected();
      resultId = link?.selectedResultId ?? null;
      return Boolean(resultId && resultId !== previousId && link?.activeRunId === null);
    });
    return resultId!;
  };
  try {
    await controls.edit('再见。', LATER_SECOND_LINE);
    const laterResultId = await waitNewResult(originalResultId);
    const later = await getTranslationComparison(projectId, translationId, laterResultId, 0, 2);
    if (
      later.segments[0].translatedLines[0] !== originalFirst ||
      later.segments[1].translatedLines[0] !== LATER_SECOND_LINE
    )
      throw new Error('First native edit changed the wrong segment');
    await checkpoint('translation-history-manual-ready');

    await controls.preview(originalResultId, originalSecond);
    if (
      (await selected())?.selectedResultId !== laterResultId ||
      !controls.button('Исправить перевод')
    )
      throw new Error('Older preview changed attachment or has no explicit branch control');
    await checkpoint('translation-history-preview');
    await controls.edit('你好。', BRANCH_FIRST_LINE);
    if (__NATIVE_E2E_TRANSLATION_HISTORY_GAP__) {
      await chooseDuringHistoricalGap(
        controls,
        projectId,
        translationId,
        originalResultId,
        originalSecond,
      );
      return;
    }
    const branchResultId = await waitNewResult(laterResultId);
    const branch = await getTranslationComparison(projectId, translationId, branchResultId, 0, 2);
    if (
      branch.segments[0].translatedLines[0] !== BRANCH_FIRST_LINE ||
      branch.segments[1].translatedLines[0] !== originalSecond
    )
      throw new Error('Older-base branch merged the later second-segment edit');
    const history = await listTranslationHistory(projectId, translationId);
    if (
      history.entries.length !== 3 ||
      history.entries[0].resultId !== branchResultId ||
      history.entries[1].resultId !== laterResultId ||
      history.entries[2].resultId !== originalResultId
    )
      throw new Error('Native branch did not retain all three immutable versions');
    await controls.wait(() =>
      Boolean(view.container.textContent?.includes('Новая версия готова и выбрана для проекта.')),
    );
    await controls.preview(branchResultId, BRANCH_FIRST_LINE);
    if (!controls.hasParagraph(originalSecond) || controls.hasParagraph(LATER_SECOND_LINE))
      throw new Error('Native comparison displayed a merged branch');
    await checkpoint('translation-history-branch-ready');

    if (__NATIVE_E2E_TRANSLATION_HISTORY_EXPORT__) {
      await exportHistoryResult(projectId, translationId, branchResultId, 'native-export');
      await exportHistoryResult(projectId, translationId, laterResultId, 'native-export');
      await checkpoint('translation-history-exported');
    }

    await controls.preview(laterResultId, LATER_SECOND_LINE);
    await controls.wait(() => Boolean(controls.button('Использовать эту версию')));
    controls.button('Использовать эту версию')!.click();
    await controls.wait(async () => {
      const link = await selected();
      const outcome = await getTranslationPublication(projectId, translationId, branchResultId);
      return (
        link?.selectedResultId === laterResultId &&
        link.activeRunId === null &&
        outcome.publication.state === 'ready' &&
        outcome.link.selectedResultId === laterResultId &&
        !controls.button('Использовать эту версию') &&
        Boolean(view.container.textContent?.includes('Новая версия готова в истории.'))
      );
    });
    await checkpoint('translation-history-selected');
  } finally {
    view.dispose();
  }
}

async function checkpoint(checkpoint: string) {
  await invoke('native_e2e_checkpoint_cmd', { checkpoint });
}
