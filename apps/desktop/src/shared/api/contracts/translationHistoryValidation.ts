import type { TranslationHistoryPage } from './translation';
import { isUuid, validateTranslationLink } from './translationValidation';
import { isNonNegativeSafeInteger, isNullable, isRecord } from './validationPrimitives';

export function validateTranslationHistoryPage(value: unknown): value is TranslationHistoryPage {
  if (
    !isRecord(value) ||
    !validateTranslationLink(value.link) ||
    !Array.isArray(value.entries) ||
    value.entries.length > 100 ||
    !isNullable(value.nextCursor, isUuid)
  )
    return false;
  const ids = new Set<string>();
  for (const entry of value.entries) {
    if (
      !isRecord(entry) ||
      !isUuid(entry.resultId) ||
      !isUuid(entry.runId) ||
      !isUuid(entry.artifactId) ||
      !isNonNegativeSafeInteger(entry.resultRevision) ||
      entry.resultRevision === 0 ||
      typeof entry.needsReview !== 'boolean' ||
      ids.has(entry.resultId)
    )
      return false;
    ids.add(entry.resultId);
  }
  return value.nextCursor === null || value.nextCursor === value.entries.at(-1)?.resultId;
}
