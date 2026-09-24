import { invoke } from '@/shared/api/tauri';
import type { TranslationComparisonPage, TranslationLink } from '@/shared/api/contracts/translation';

export function listProjectTranslations(projectId: string): Promise<TranslationLink[]> {
  return invoke('list_project_translations_cmd', { projectId });
}

export function getTranslationComparison(
  projectId: string,
  translationId: string,
  resultId: string,
  offset: number,
  limit: number,
): Promise<TranslationComparisonPage> {
  return invoke('get_translation_comparison_cmd', {
    projectId,
    translationId,
    resultId,
    offset,
    limit,
  });
}
