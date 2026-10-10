import {
  editHistoricalTranslationResult,
  getHistoricalTranslationEditContext,
  listTranslationHistory,
  publishTranslationResult,
} from '@/entities/translation';
import type { EditHistoricalTranslationResult } from '@/shared/api/contracts/translationHistoricalEdit';
import { invoke } from '@/shared/api/tauri';
import type { TranslationHistoryControls } from './translationHistoryControls';

export async function retryHistoricalJournal(
  controls: TranslationHistoryControls,
  projectId: string,
  translationId: string,
): Promise<void> {
  const history = await listTranslationHistory(projectId, translationId);
  if (
    history.entries.length !== 2 ||
    history.link.selectedResultId !== history.entries[1].resultId ||
    history.link.revision !== 4
  )
    throw new Error('Journal-only restart invented a result or changed the explicit choice');
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-history-journal-reopened' });
  let input: EditHistoricalTranslationResult | null = null;
  await controls.wait(async () => {
    input = await invoke('native_e2e_historical_retry_input_cmd');
    return input !== null;
  });
  const frozen = input!;
  if (
    frozen.projectId !== projectId ||
    frozen.translationId !== translationId ||
    frozen.baseResultId !== history.entries[1].resultId ||
    frozen.expectedHeadResultId !== history.entries[0].resultId ||
    frozen.expectedLinkRevision !== 3
  )
    throw new Error('Journal retry changed the originally admitted request observations');
  const context = await getHistoricalTranslationEditContext(
    projectId,
    translationId,
    frozen.baseResultId,
  );
  if (
    context.expectedHeadResultId !== frozen.expectedHeadResultId ||
    context.headRevision !== 2 ||
    context.expectedLinkRevision !== 4
  )
    throw new Error('Journal-only startup changed the Translate head or current link');
  const first = await editHistoricalTranslationResult(frozen);
  const replay = await editHistoricalTranslationResult(frozen);
  if (first !== frozen.resultId || replay !== first)
    throw new Error('Exact journal retry created different result identities');
  const publication = await publishTranslationResult(projectId, translationId, first);
  const repeated = await publishTranslationResult(projectId, translationId, first);
  if (publication.resultId !== first || repeated.artifactId !== publication.artifactId)
    throw new Error('Repeated journal publication allocated another artifact');
  await invoke('native_e2e_checkpoint_cmd', { checkpoint: 'translation-history-journal-retried' });
}
