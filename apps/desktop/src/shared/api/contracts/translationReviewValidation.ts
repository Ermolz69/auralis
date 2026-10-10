import type { TranslationReviewView } from './translation';
import { isSafeRevision } from './jobValidation';
import { isUuid } from './translationValidation';
import { isNullable, isRecord, isString, isTimestamp } from './validationPrimitives';

export function validateTranslationReviewView(value: unknown): value is TranslationReviewView {
  if (!isRecord(value)) return false;
  return (
    isUuid(value.resultId) &&
    isString(value.resultSha256) &&
    /^[0-9a-f]{64}$/.test(value.resultSha256) &&
    isSafeRevision(value.linkRevision) &&
    typeof value.needsReview === 'boolean' &&
    typeof value.selected === 'boolean' &&
    isNullable(
      value.latest,
      (latest) =>
        isRecord(latest) &&
        isUuid(latest.eventId) &&
        (latest.decision === 'approved' || latest.decision === 'withdrawn') &&
        isString(latest.reviewerLabel) &&
        latest.reviewerLabel.trim().length > 0 &&
        isString(latest.note) &&
        isTimestamp(latest.createdAt),
    )
  );
}
