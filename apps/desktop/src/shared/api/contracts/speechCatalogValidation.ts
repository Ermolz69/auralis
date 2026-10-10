import type { SpeechCatalogSnapshot } from './speechCatalog';
import { isArrayOf, isRecord, isString } from './validationPrimitives';

const isHash = (value: unknown): value is string =>
  isString(value) && /^[0-9a-f]{64}$/.test(value);

export function validateSpeechCatalog(value: unknown): value is SpeechCatalogSnapshot {
  return isRecord(value) &&
    isArrayOf(value.profiles, (profile) => isRecord(profile) &&
      isString(profile.profile_id) && isHash(profile.profile_sha256) &&
      isString(profile.engine_id)) &&
    isArrayOf(value.voices, (voice) => isRecord(voice) &&
      isString(voice.voice_id) && isString(voice.display_name) &&
      isString(voice.description) && isHash(voice.manifest_sha256) &&
      isArrayOf(voice.references, (reference) => isRecord(reference) &&
        isString(reference.reference_id) && isString(reference.engine_id) &&
        isString(reference.emotion_id) && isString(reference.transcript) &&
        isHash(reference.audio_sha256) &&
        ['unverified', 'qualified', 'rejected'].includes(String(reference.qualification_status))));
}
