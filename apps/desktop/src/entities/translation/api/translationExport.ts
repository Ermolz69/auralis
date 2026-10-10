import { invoke } from '@/shared/api/tauri';
import type {
  TranslationExportReceipt,
  TranslationReviewView,
} from '@/shared/api/contracts/translation';

export async function exportTranslationResult(
  projectId: string,
  translationId: string,
  view: TranslationReviewView,
  destination: string,
): Promise<TranslationExportReceipt> {
  const receipt = await invoke('export_translation_result_cmd', {
    projectId,
    translationId,
    resultId: view.resultId,
    expectedResultSha256: view.resultSha256,
    destination,
  });
  if (
    receipt.resultId !== view.resultId ||
    receipt.resultSha256 !== view.resultSha256 ||
    receipt.destination !== destination
  ) {
    throw new Error('Экспортированная версия отличается от выбранной');
  }
  return receipt;
}
