import type { TranslationExportReceipt } from './translation';
import { isUuid } from './translationValidation';
import { isRecord, isString } from './validationPrimitives';

export function validateTranslationExportReceipt(
  value: unknown,
): value is TranslationExportReceipt {
  return (
    isRecord(value) &&
    isUuid(value.resultId) &&
    isString(value.resultSha256) &&
    /^[0-9a-f]{64}$/.test(value.resultSha256) &&
    typeof value.sizeBytes === 'number' &&
    Number.isSafeInteger(value.sizeBytes) &&
    value.sizeBytes >= 0 &&
    isString(value.destination) &&
    value.destination.length > 0
  );
}
