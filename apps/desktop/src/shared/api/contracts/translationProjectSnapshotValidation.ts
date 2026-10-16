import type { ProjectTranslationSnapshot } from './translation';
import { isRecord } from './validationPrimitives';
import {
  isUuid,
  validateTranslationLink,
  validateTranslationRunSnapshot,
} from './translationValidation';

export function validateProjectTranslationSnapshot(
  value: unknown,
): value is ProjectTranslationSnapshot {
  if (!isRecord(value) || !isUuid(value.projectId) || !Array.isArray(value.entries)) return false;
  const translationIds = new Set<string>();
  for (const item of value.entries) {
    if (!isRecord(item) || !validateTranslationLink(item.link)) return false;
    const { link, run } = item;
    if (link.projectId !== value.projectId || translationIds.has(link.translationId)) return false;
    translationIds.add(link.translationId);
    if (link.activeRunId === null) {
      if (run !== null) return false;
    } else if (!validateTranslationRunSnapshot(run) || run.runId !== link.activeRunId) {
      return false;
    }
  }
  return true;
}
