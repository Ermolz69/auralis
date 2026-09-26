import type { EditHistoricalTranslationResult } from '@/shared/api/contracts/translationHistoricalEdit';

export interface HistoricalEditAttempt {
  input: EditHistoricalTranslationResult;
  runId: string;
  edited: boolean;
}
