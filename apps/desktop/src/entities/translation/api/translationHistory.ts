import { invoke } from '@/shared/api/tauri';
import type { TranslationHistoryPage, TranslationLink } from '@/shared/api/contracts/translation';

export async function listTranslationHistory(
  projectId: string,
  translationId: string,
  beforeResultId: string | null = null,
  limit = 25,
): Promise<TranslationHistoryPage> {
  const page = await invoke('list_translation_history_cmd', {
    projectId,
    translationId,
    beforeResultId,
    limit,
  });
  if (page.link.projectId !== projectId || page.link.translationId !== translationId)
    throw new Error('Translation history belongs to another project');
  return page;
}

export async function selectTranslationResult(
  projectId: string,
  translationId: string,
  resultId: string,
  expectedLinkRevision: number,
): Promise<TranslationLink> {
  const link = await invoke('select_translation_result_cmd', {
    projectId,
    translationId,
    resultId,
    expectedLinkRevision,
  });
  if (
    link.projectId !== projectId ||
    link.translationId !== translationId ||
    link.selectedResultId !== resultId
  )
    throw new Error('Selected translation differs from the requested version');
  return link;
}
