import { listProjectTranslations } from '@/entities/translation';
import { invoke } from '@/shared/api/tauri';
import type { TranslationHistoryControls } from './translationHistoryControls';

export async function chooseDuringHistoricalGap(
  controls: TranslationHistoryControls,
  projectId: string,
  translationId: string,
  originalResultId: string,
  originalSecond: string,
): Promise<void> {
  await controls.wait(() => invoke('native_e2e_historical_edit_held_cmd'));
  await controls.preview(originalResultId, originalSecond);
  await controls.wait(() => Boolean(controls.button('Использовать эту версию')));
  controls.button('Использовать эту версию')!.click();
  await controls.wait(async () => {
    const link = (await listProjectTranslations(projectId)).find(
      (item) => item.translationId === translationId,
    );
    return (
      link?.selectedResultId === originalResultId && !controls.button('Использовать эту версию')
    );
  });
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-history-gap-choice' });
  await new Promise<never>(() => undefined);
}
