import { isNonNegativeSafeInteger, isRecord, isString } from './validationPrimitives';

export function validateTranscript(value: unknown): boolean {
  if (!isRecord(value) || !isString(value.language) || !Array.isArray(value.segments)) return false;
  const ids = new Set<string>();
  const indexes = new Set<number>();
  for (const segment of value.segments) {
    if (
      !isRecord(segment) ||
      !isString(segment.id) ||
      !isNonNegativeSafeInteger(segment.index) ||
      !isNonNegativeSafeInteger(segment.startMs) ||
      !isNonNegativeSafeInteger(segment.endMs) ||
      segment.endMs < segment.startMs ||
      !isString(segment.sourceText) ||
      ids.has(segment.id) ||
      indexes.has(segment.index)
    )
      return false;
    ids.add(segment.id);
    indexes.add(segment.index);
  }
  return true;
}
