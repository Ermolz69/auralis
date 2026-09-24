import type { TranslationInspection } from './translation';
import {
  isNonNegativeSafeInteger,
  isNullable,
  isRecord,
  isString,
} from './validationPrimitives';

export function validateTranslationInspection(value: unknown): value is TranslationInspection {
  if (
    !isRecord(value) ||
    (value.format !== 'srt' && value.format !== 'vtt') ||
    !isString(value.sourceSha256) ||
    !/^[0-9a-f]{64}$/.test(value.sourceSha256) ||
    !isNonNegativeSafeInteger(value.sourceSize) ||
    !isNonNegativeSafeInteger(value.parserVersion) ||
    value.parserVersion === 0 ||
    !Array.isArray(value.segments)
  ) {
    return false;
  }
  const sourceSize = value.sourceSize;
  return value.segments.every((segment) => {
    if (
      !isRecord(segment) ||
      !isNonNegativeSafeInteger(segment.id) ||
      !isNonNegativeSafeInteger(segment.ordinal) ||
      !isNullable(segment.cueLabel, isString) ||
      !isNonNegativeSafeInteger(segment.startMs) ||
      !isNonNegativeSafeInteger(segment.endMs) ||
      segment.endMs < segment.startMs ||
      !Array.isArray(segment.sourceLines) ||
      !segment.sourceLines.every(isString) ||
      !Array.isArray(segment.textRanges) ||
      segment.textRanges.length !== segment.sourceLines.length
    ) {
      return false;
    }
    return segment.textRanges.every(
      (range) =>
        Array.isArray(range) &&
        range.length === 2 &&
        isNonNegativeSafeInteger(range[0]) &&
        isNonNegativeSafeInteger(range[1]) &&
        range[0] < range[1] &&
        range[1] <= sourceSize,
    );
  });
}
