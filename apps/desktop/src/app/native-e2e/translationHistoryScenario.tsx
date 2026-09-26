import {
  getTranslationComparison,
  listProjectTranslations,
  listTranslationHistory,
} from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';
import { mountTranslationReview } from './translationReviewScenarioView';

const TIMEOUT_MS = 60_000;
const POLL_INTERVAL_MS = 25;
const MANUAL_TEXT = 'Привет, мир.';

export async function translationHistoryScenario(
  projectId: string,
  translationId: string,
  originalResultId: string,
): Promise<void> {
  const original = await getTranslationComparison(projectId, translationId, originalResultId, 0, 1);
  const originalText = original.segments[0].translatedLines[0];
  if (originalText === MANUAL_TEXT)
    throw new Error('Manual edit must differ from the model result');
  const view = mountTranslationReview(projectId);
  const deadline = Date.now() + TIMEOUT_MS;
  const waitFor = async (predicate: () => boolean | Promise<boolean>) => {
    while (Date.now() < deadline) {
      if (await predicate()) return;
      await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    }
    throw new Error(`History controls timed out: ${view.container.textContent?.slice(-1200)}`);
  };
  const button = (label: string) =>
    Array.from(view.container.querySelectorAll('button')).find(
      (item) => item.textContent?.trim() === label && !item.disabled,
    );
  try {
    await waitFor(() => Boolean(button('Исправить перевод')));
    button('Исправить перевод')!.click();
    await waitFor(() => Boolean(view.container.querySelector('input[type="text"]')));
    const input = view.container.querySelector<HTMLInputElement>('input[type="text"]')!;
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    if (!setter) throw new Error('Native input setter is unavailable');
    setter.call(input, MANUAL_TEXT);
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() => Boolean(button('Сохранить новую версию')));
    button('Сохранить новую версию')!.click();
    let editedResultId: string | null = null;
    await waitFor(async () => {
      const link = (await listProjectTranslations(projectId)).find(
        (item) => item.translationId === translationId,
      );
      if (!link?.selectedResultId || link.selectedResultId === originalResultId) return false;
      editedResultId = link.selectedResultId;
      return Boolean(view.container.textContent?.includes(MANUAL_TEXT));
    });
    const history = await listTranslationHistory(projectId, translationId);
    if (
      history.entries.length !== 2 ||
      history.entries[0].resultId !== editedResultId ||
      history.entries[1].resultId !== originalResultId ||
      history.link.activeRunId !== null
    )
      throw new Error('Manual edit did not produce two ready historical versions');
    const edited = await getTranslationComparison(projectId, translationId, editedResultId!, 0, 1);
    if (
      edited.segments[0].translatedLines[0] !== MANUAL_TEXT ||
      edited.segments[0].sourceLines[0] !== '你好。'
    )
      throw new Error('Actual input change was not saved to the new result');
    await checkpoint('translation-history-manual-ready');
    const select = view.container.querySelector<HTMLSelectElement>(
      '#translation-history-selection',
    );
    if (!select) throw new Error('Native history selector is missing');
    await waitFor(
      () =>
        !select.disabled &&
        Array.from(select.options).some((item) => item.value === originalResultId),
    );
    select.value = originalResultId;
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(
      () =>
        Boolean(button('Использовать эту версию')) &&
        Array.from(view.container.querySelectorAll('p')).some(
          (item) => item.textContent === originalText,
        ),
    );
    const previewed = (await listProjectTranslations(projectId)).find(
      (item) => item.translationId === translationId,
    );
    if (previewed?.selectedResultId !== editedResultId || button('Исправить перевод'))
      throw new Error('History preview changed attachment or enabled a stale edit');
    await checkpoint('translation-history-preview');
    button('Использовать эту версию')!.click();
    await waitFor(async () => {
      const link = (await listProjectTranslations(projectId)).find(
        (item) => item.translationId === translationId,
      );
      return (
        link?.selectedResultId === originalResultId &&
        link.activeRunId === null &&
        !button('Использовать эту версию')
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
