import { invoke } from '@/shared/api/tauri';
import type {
  EditTranslationResult,
  TranslationComparisonPage,
  TranslationLink,
  TranslationPublication,
} from '@/shared/api/contracts/translation';

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

export function editTranslationResult(input: EditTranslationResult): Promise<string> {
  return invoke('edit_translation_result_cmd', { input });
}

export function publishTranslationResult(
  projectId: string,
  translationId: string,
  resultId: string,
): Promise<TranslationPublication> {
  return invoke('publish_translation_result_cmd', { projectId, translationId, resultId });
}
