import type {
  CastResult,
  ReferencePlayback,
  VoiceCatalogPage,
  VoiceCompatibility,
  VoiceDetails,
} from './caster';
import {
  isArrayOf,
  isBoolean,
  isNonNegativeSafeInteger,
  isNullable,
  isRecord,
  isString,
} from './validationPrimitives';

const hash = (value: unknown): value is string => isString(value) && /^[0-9a-f]{64}$/.test(value);
const identifier = (value: unknown): value is string =>
  isString(value) && /^[A-Za-z0-9_-]{1,64}$/.test(value);
const capability = (value: unknown): boolean =>
  isRecord(value) && identifier(value.engine_id) && identifier(value.emotion_id);
const reference = (value: unknown): boolean =>
  isRecord(value) &&
  identifier(value.reference_id) &&
  identifier(value.engine_id) &&
  identifier(value.emotion_id) &&
  isString(value.transcript) &&
  hash(value.audio_sha256) &&
  (value.qualification_status === 'unverified' ||
    value.qualification_status === 'qualified' ||
    value.qualification_status === 'rejected') &&
  isNullable(value.qualification_report_sha256, hash);

export function validateVoiceCatalogPage(value: unknown): value is VoiceCatalogPage {
  return (
    isRecord(value) &&
    isNullable(value.next_after, identifier) &&
    isArrayOf(
      value.items,
      (entry) =>
        isRecord(entry) &&
        identifier(entry.voice_id) &&
        (entry.state === 'valid' || entry.state === 'legacy' || entry.state === 'invalid') &&
        isNullable(entry.display_name, isString) &&
        isNullable(entry.description, isString) &&
        isNullable(entry.manifest_sha256, hash) &&
        isNullable(entry.reference_count, isNonNegativeSafeInteger) &&
        isNullable(entry.qualified_capabilities, (capabilities) =>
          isArrayOf(capabilities, capability),
        ) &&
        isNullable(entry.error_code, isString),
    ) &&
    Array.isArray(value.items) &&
    value.items.length <= 50
  );
}

export function validateVoiceDetails(value: unknown): value is VoiceDetails {
  return (
    isRecord(value) &&
    identifier(value.voice_id) &&
    isString(value.display_name) &&
    isString(value.description) &&
    hash(value.manifest_sha256) &&
    isArrayOf(value.references, reference) &&
    Array.isArray(value.references) &&
    value.references.length <= 50 &&
    isNullable(value.next_after_reference, identifier)
  );
}

export function validateReferencePlayback(value: unknown): value is ReferencePlayback {
  return (
    reference(value) &&
    isRecord(value) &&
    identifier(value.voice_id) &&
    isString(value.playback_path)
  );
}

export function validateVoiceCompatibility(value: unknown): value is VoiceCompatibility {
  return (
    capability(value) &&
    isRecord(value) &&
    identifier(value.voice_id) &&
    isBoolean(value.compatible) &&
    isArrayOf(value.reference_ids, identifier)
  );
}

const castRole = (value: unknown): boolean =>
  isRecord(value) && isString(value.display_name) && isNullable(value.voice_id, identifier);
const logicalCast = (value: unknown): boolean =>
  isRecord(value) &&
  value.version === 2 &&
  isRecord(value.roles) &&
  Object.entries(value.roles).every(([id, role]) => identifier(id) && castRole(role));
const snapshot = (value: unknown): boolean =>
  isRecord(value) &&
  identifier(value.voice_id) &&
  identifier(value.reference_id) &&
  capability(value) &&
  hash(value.manifest_sha256) &&
  hash(value.audio_sha256) &&
  hash(value.report_sha256) &&
  isString(value.transcript);

export function validateCastResult(value: unknown): value is CastResult {
  if (!isRecord(value) || !hash(value.revision_sha256) || !isRecord(value.revision)) return false;
  const revision = value.revision;
  return (
    revision.version === 1 &&
    isNullable(revision.parent_revision, hash) &&
    identifier(revision.request_id) &&
    hash(revision.request_sha256) &&
    logicalCast(revision.cast) &&
    isRecord(revision.selections) &&
    Object.entries(revision.selections).every(
      ([id, selected]) => identifier(id) && snapshot(selected),
    )
  );
}
