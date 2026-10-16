import type {
  TranslationComparisonPage,
  TranslationInspection,
  TranslationLink,
  TranslationPackageStatus,
  TranslationPublication,
  TranslationRunSnapshot,
  TranslationStart,
} from './translation';
import {
  isNonNegativeSafeInteger,
  isNullable,
  isRecord,
  isString,
  isTimestamp,
} from './validationPrimitives';

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
export const isUuid = (value: unknown): value is string => isString(value) && uuid.test(value);

export function validateTranslationPackageStatus(
  value: unknown,
): value is TranslationPackageStatus {
  return (
    isRecord(value) &&
    value.schemaVersion === 1 &&
    isString(value.releaseId) &&
    value.releaseId.length > 0 &&
    isString(value.backend) &&
    value.backend.length > 0 &&
    typeof value.supported === 'boolean' &&
    typeof value.installed === 'boolean' &&
    typeof value.selected === 'boolean' &&
    (!value.selected || value.installed)
  );
}

export function validateTranslationStart(value: unknown): value is TranslationStart {
  return (
    isRecord(value) && isUuid(value.jobId) && isUuid(value.runId) && value.state === 'accepted'
  );
}

export function validateTranslationComparisonPage(
  value: unknown,
): value is TranslationComparisonPage {
  if (
    !isRecord(value) ||
    !isUuid(value.resultId) ||
    !isUuid(value.runId) ||
    typeof value.needsReview !== 'boolean' ||
    !isNonNegativeSafeInteger(value.totalSegments) ||
    !isNonNegativeSafeInteger(value.offset) ||
    !Array.isArray(value.segments) ||
    value.segments.length > 100 ||
    value.segments.length > Math.max(0, value.totalSegments - value.offset)
  ) {
    return false;
  }
  return value.segments.every(
    (segment) =>
      isRecord(segment) &&
      isNonNegativeSafeInteger(segment.id) &&
      segment.id > 0 &&
      isNonNegativeSafeInteger(segment.ordinal) &&
      isNullable(segment.cueLabel, isString) &&
      isNonNegativeSafeInteger(segment.startMs) &&
      isNonNegativeSafeInteger(segment.endMs) &&
      segment.endMs >= segment.startMs &&
      Array.isArray(segment.sourceLines) &&
      Array.isArray(segment.translatedLines) &&
      segment.sourceLines.length > 0 &&
      segment.sourceLines.length === segment.translatedLines.length &&
      segment.sourceLines.every(isString) &&
      segment.translatedLines.every(isString),
  );
}

export function validateTranslationPublication(value: unknown): value is TranslationPublication {
  return (
    isRecord(value) &&
    isUuid(value.translationId) &&
    isUuid(value.resultId) &&
    isUuid(value.runId) &&
    isUuid(value.artifactId) &&
    isString(value.sourceSha256) &&
    /^[0-9a-f]{64}$/.test(value.sourceSha256) &&
    typeof value.needsReview === 'boolean' &&
    (value.state === 'pending' || value.state === 'ready' || value.state === 'failed')
  );
}

export function validateTranslationLink(value: unknown): value is TranslationLink {
  return (
    isRecord(value) &&
    isUuid(value.translationId) &&
    isUuid(value.projectId) &&
    isUuid(value.sourceArtifactId) &&
    isString(value.sourceSha256) &&
    /^[0-9a-f]{64}$/.test(value.sourceSha256) &&
    (value.sourceFormat === 'srt' || value.sourceFormat === 'vtt') &&
    (value.sourceLanguage === 'zh' || value.sourceLanguage === 'ja') &&
    value.targetLanguage === 'ru' &&
    isNullable(value.activeRunId, isUuid) &&
    isNullable(value.selectedResultId, isUuid) &&
    isNullable(value.outputArtifactId, isUuid) &&
    (value.selectedResultId === null) === (value.outputArtifactId === null) &&
    isNonNegativeSafeInteger(value.revision) &&
    value.revision > 0 &&
    isTimestamp(value.createdAt) &&
    isTimestamp(value.updatedAt)
  );
}

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

const runPhases = new Set(['requested', 'running', 'paused', 'failed', 'validated']);

export function validateTranslationRunSnapshot(value: unknown): value is TranslationRunSnapshot {
  if (
    !isRecord(value) ||
    !isUuid(value.runId) ||
    !isString(value.phase) ||
    !runPhases.has(value.phase) ||
    typeof value.pauseRequested !== 'boolean' ||
    !isNonNegativeSafeInteger(value.savedBlocks) ||
    !isNonNegativeSafeInteger(value.totalBlocks) ||
    value.savedBlocks > value.totalBlocks ||
    !isNullable(value.resultId, isString) ||
    !isNullable(value.needsReview, (item) => typeof item === 'boolean') ||
    !isNullable(value.activeHostJobId, isUuid)
  ) {
    return false;
  }
  if (value.phase === 'validated') {
    return value.resultId !== null && value.needsReview !== null;
  }
  return value.resultId === null && value.needsReview === null;
}
