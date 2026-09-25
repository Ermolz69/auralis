import { invoke } from '@/shared/api/tauri';
import type { Artifact } from '@/shared/api/contracts/artifact';
import type {
  BeginTranslationRun,
  EditTranslationResult,
  TranslationComparisonPage,
  TranslationFormat,
  TranslationLink,
  TranslationPublication,
} from '@/shared/api/contracts/translation';

export function beginTranslationRun(input: BeginTranslationRun): Promise<TranslationLink> {
  return invoke('begin_translation_run_cmd', { input });
}

export function importTranslationSource(
  projectId: string,
  path: string,
  format: TranslationFormat,
): Promise<Artifact> {
  return invoke('import_translation_source_cmd', { projectId, path, format });
}

export function listOriginalSubtitles(projectId: string): Promise<Artifact[]> {
  return invoke('list_project_artifacts_cmd', { projectId, kind: 'originalSubtitle' });
}

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
