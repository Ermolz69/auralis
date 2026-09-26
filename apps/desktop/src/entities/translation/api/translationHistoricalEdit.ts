import { invoke } from '@/shared/api/tauri';
import type {
  EditHistoricalTranslationResult,
  HistoricalTranslationEditContext,
  TranslationPublicationView,
} from '@/shared/api/contracts/translationHistoricalEdit';

export async function getHistoricalTranslationEditContext(
  projectId: string,
  translationId: string,
  baseResultId: string,
): Promise<HistoricalTranslationEditContext> {
  const context = await invoke('get_historical_translation_edit_context_cmd', {
    projectId,
    translationId,
    baseResultId,
  });
  if (
    context.projectId !== projectId ||
    context.translationId !== translationId ||
    context.baseResultId !== baseResultId
  )
    throw new Error('Historical edit context differs from the requested base');
  return context;
}

export async function editHistoricalTranslationResult(
  input: EditHistoricalTranslationResult,
): Promise<string> {
  const resultId = await invoke('edit_historical_translation_result_cmd', { input });
  if (resultId !== input.resultId)
    throw new Error('Committed historical result differs from the request');
  return resultId;
}

export async function getTranslationPublication(
  projectId: string,
  translationId: string,
  resultId: string,
): Promise<TranslationPublicationView> {
  const view = await invoke('get_translation_publication_cmd', {
    projectId,
    translationId,
    resultId,
  });
  if (
    view.link.projectId !== projectId ||
    view.link.translationId !== translationId ||
    view.publication.resultId !== resultId
  )
    throw new Error('Publication view differs from the requested project result');
  return view;
}
