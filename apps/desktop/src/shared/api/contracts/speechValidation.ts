import type { SpeechCommandMap } from './speech';
import type { Validator } from './validationPrimitives';
import { validateSpeechCatalog } from './speechCatalogValidation';
import {
  isArrayOf,
  isNonNegativeSafeInteger,
  isNullable,
  isRecord,
  isString,
} from './validationPrimitives';
import { validateJobDto } from './jobValidation';

const digest = (value: unknown) => isString(value) && /^[a-f0-9]{64}$/.test(value);
export function validateSpeechSource(value: unknown): boolean {
  return (
    isRecord(value) &&
    isString(value.kind) &&
    isNullable(value.source_id, isString) &&
    isString(value.revision) &&
    digest(value.sha256) &&
    isString(value.text) &&
    isArrayOf(
      value.cues,
      (cue) =>
        isRecord(cue) &&
        isString(cue.id) &&
        isString(cue.text) &&
        isString(cue.role) &&
        isNonNegativeSafeInteger(cue.start_ms) &&
        isNonNegativeSafeInteger(cue.end_ms) &&
        cue.end_ms > cue.start_ms,
    )
  );
}
export function validateSpeechReview(value: unknown): boolean {
  return (
    isRecord(value) &&
    isString(value.decision_id) &&
    isString(value.project_id) &&
    (value.original_source === undefined ||
      isNullable(value.original_source, validateSpeechSource)) &&
    validateSpeechSource(value.source)
  );
}
export function validateSpeechRun(value: unknown): boolean {
  return (
    isRecord(value) &&
    isString(value.run_id) &&
    isString(value.profile_id) &&
    isNonNegativeSafeInteger(value.utterance_count) &&
    value.utterance_count > 0 &&
    isNonNegativeSafeInteger(value.chunk_count) &&
    value.chunk_count >= value.utterance_count &&
    isNullable(value.job, validateJobDto) &&
    isNullable(
      value.result,
      (result) =>
        isRecord(result) &&
        isRecord(value.job) &&
        value.job.status === 'completed' &&
        result.ready === true &&
        isString(result.run_id) &&
        result.run_id === value.run_id &&
        isString(result.attempt_id) &&
        isNonNegativeSafeInteger(result.duration_ms) &&
        result.duration_ms > 0 &&
        isArrayOf(
          result.files,
          (file) =>
            isRecord(file) &&
            isString(file.artifact_id) &&
            isString(file.kind) &&
            ['wav', 'manifest', 'segment', 'audio_layout'].includes(file.kind) &&
            digest(file.sha256) &&
            isNonNegativeSafeInteger(file.size_bytes) &&
            file.size_bytes > 0,
        ) &&
        Array.isArray(result.files) &&
        result.files.filter((f) => isRecord(f) && f.kind === 'audio_layout').length <= 1 &&
        result.files.length ===
          Number(value.chunk_count) +
            2 +
            result.files.filter((f) => isRecord(f) && f.kind === 'audio_layout').length &&
        result.files.filter((f) => isRecord(f) && f.kind === 'wav').length === 1 &&
        result.files.filter((f) => isRecord(f) && f.kind === 'manifest').length === 1 &&
        result.files.filter((f) => isRecord(f) && f.kind === 'segment').length ===
          value.chunk_count &&
        new Set(result.files.map((f) => (isRecord(f) ? f.artifact_id : null))).size ===
          result.files.length,
    )
  );
}

export const speechCommandResultValidators = {
  get_speech_voice_selection_cmd: (value) => isNullable(value, validateVoiceBinding),
  save_speech_voice_selection_cmd: validateVoiceBinding,
  prepare_speech_text_cmd: validateMarkerPreparation,
  review_prepared_speech_text_cmd: validateSpeechReview,
  get_speech_catalog_cmd: validateSpeechCatalog,
  get_speech_source_cmd: validateSpeechSource,
  review_speech_text_cmd: validateSpeechReview,
  prepare_speech_plan_cmd: validateSpeechRun,
  start_speech_run_cmd: validateSpeechRun,
  resume_speech_run_cmd: validateSpeechRun,
  get_speech_run_cmd: validateSpeechRun,
  list_speech_runs_cmd: (value) => isArrayOf(value, validateSpeechRun),
} satisfies Record<keyof SpeechCommandMap | 'get_speech_catalog_cmd', Validator>;

function validateMarkerPreparation(value: unknown): boolean {
  return (
    isRecord(value) &&
    value.version === '1.0.0' &&
    validateSpeechSource(value.original) &&
    validateSpeechSource(value.prepared) &&
    isNonNegativeSafeInteger(value.changed_targets) &&
    isNonNegativeSafeInteger(value.unresolved_targets)
  );
}
function validateVoiceBinding(value: unknown): boolean {
  return (
    isRecord(value) &&
    isString(value.role_id) &&
    isString(value.display_name) &&
    isString(value.voice_id) &&
    isString(value.reference_id) &&
    isString(value.engine_id)
  );
}
