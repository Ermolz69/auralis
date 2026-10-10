import type {
  HistoricalTranslationEditContext,
  EditHistoricalTranslationResult,
  TranslationPublicationView,
} from './translationHistoricalEdit';
import {
  isUuid,
  validateTranslationLink,
  validateTranslationPublication,
} from './translationValidation';
import { isNonNegativeSafeInteger, isRecord } from './validationPrimitives';

const NIL_UUID = '00000000-0000-0000-0000-000000000000';
const nonNilUuid = (value: unknown) => isUuid(value) && value !== NIL_UUID;
const positiveInteger = (value: unknown): value is number =>
  isNonNegativeSafeInteger(value) && value > 0;

export function validateHistoricalTranslationEditRequest(
  value: unknown,
): value is EditHistoricalTranslationResult {
  return (
    isRecord(value) &&
    [
      value.projectId,
      value.translationId,
      value.baseResultId,
      value.resultId,
      value.expectedHeadResultId,
    ].every(nonNilUuid) &&
    positiveInteger(value.expectedLinkRevision) &&
    positiveInteger(value.segmentId) &&
    value.segmentId <= 0xffffffff &&
    Array.isArray(value.lines) &&
    value.lines.length > 0 &&
    value.lines.every((line) => typeof line === 'string')
  );
}

export function validateHistoricalTranslationEditContext(
  value: unknown,
): value is HistoricalTranslationEditContext {
  return (
    isRecord(value) &&
    [
      value.projectId,
      value.translationId,
      value.baseResultId,
      value.runId,
      value.expectedHeadResultId,
    ].every(nonNilUuid) &&
    positiveInteger(value.baseRevision) &&
    positiveInteger(value.headRevision) &&
    value.headRevision >= value.baseRevision &&
    positiveInteger(value.expectedLinkRevision) &&
    (value.baseResultId === value.expectedHeadResultId) ===
      (value.baseRevision === value.headRevision)
  );
}

export function validateTranslationPublicationView(
  value: unknown,
): value is TranslationPublicationView {
  if (
    !isRecord(value) ||
    !validateTranslationLink(value.link) ||
    !validateTranslationPublication(value.publication)
  )
    return false;
  const { link, publication } = value;
  return (
    link.translationId === publication.translationId &&
    link.sourceSha256 === publication.sourceSha256 &&
    (link.selectedResultId !== publication.resultId ||
      (publication.state === 'ready' && link.outputArtifactId === publication.artifactId))
  );
}
