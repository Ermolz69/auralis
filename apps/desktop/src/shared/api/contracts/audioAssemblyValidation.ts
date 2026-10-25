import type { AudioAssemblyCommandMap } from './audioAssembly';
import { validateTimingConflict } from './audioConflictValidation';
import { validateJobDto } from './jobValidation';
import {
  isArrayOf,
  isBoolean,
  isNonNegativeSafeInteger as integer,
  isNullable,
  isRecord,
  isString,
  type Validator,
} from './validationPrimitives';

const digest = (value: unknown) => isString(value) && /^[a-f0-9]{64}$/.test(value);
const id = (value: unknown) => isString(value) && value.length > 0;
const positive = (value: unknown): value is number => integer(value) && value > 0;
const channels = (value: unknown) => value === 1 || value === 2;
function videoSource(value: unknown): boolean {
  if (value === undefined || value === null) return true;
  return (
    isRecord(value) &&
    value.policy_id === 'replace_original_audio_pad_silence_h264_aac_v1' &&
    isRecord(value.artifact) &&
    isRecord(value.media) &&
    id(value.artifact.artifact_id) &&
    digest(value.artifact.sha256) &&
    positive(value.artifact.size_bytes) &&
    value.artifact.size_bytes === value.media.size_bytes &&
    value.artifact.sha256 === value.media.sha256 &&
    positive(value.media.duration_ms) &&
    value.media.duration_ms <= 28800000 &&
    positive(value.media.width) &&
    positive(value.media.height) &&
    isString(value.media.video_codec) &&
    digest(value.media.ffprobe_sha256)
  );
}
function settings(value: unknown): boolean {
  return (
    isRecord(value) &&
    ['continuous', 'subtitle_window'].includes(String(value.mode)) &&
    isNullable(value.source_duration_ms, positive) &&
    isArrayOf(value.pause_after_ms, integer) &&
    Array.isArray(value.pause_after_ms) &&
    value.pause_after_ms.length > 0 &&
    value.pause_after_ms.length <= 8192
  );
}
function measurement(value: unknown): boolean {
  return (
    isRecord(value) &&
    id(value.utterance_id) &&
    digest(value.audio_sha256) &&
    value.sample_rate === 48000 &&
    channels(value.channels) &&
    positive(value.frame_count) &&
    integer(value.pause_after_ms) &&
    isNullable(
      value.source_window,
      (window) => isRecord(window) && integer(window.start_ms) && integer(window.end_ms),
    )
  );
}
export function validateAssemblyPreview(value: unknown): boolean {
  if (
    !isRecord(value) ||
    !isRecord(value.settings) ||
    !Array.isArray(value.utterances) ||
    !Array.isArray(value.conflicts)
  )
    return false;
  return (
    id(value.source_run_id) &&
    videoSource(value.video) &&
    digest(value.source_plan_sha256) &&
    digest(value.preview_sha256) &&
    id(value.profile_id) &&
    settings(value.settings) &&
    value.sample_rate === 48000 &&
    channels(value.channels) &&
    isNullable(value.frames, positive) &&
    isArrayOf(value.utterances, measurement) &&
    value.utterances.length > 0 &&
    value.utterances.length <= 8192 &&
    Array.isArray(value.settings.pause_after_ms) &&
    value.settings.pause_after_ms.length === value.utterances.length &&
    new Set(value.utterances.map((item) => (isRecord(item) ? item.utterance_id : null))).size ===
      value.utterances.length &&
    isArrayOf(value.conflicts, validateTimingConflict) &&
    (value.conflicts.length === 0 ? positive(value.frames) : value.frames === null) &&
    isBoolean(value.renderer_available) &&
    isNullable(value.renderer_error, isString)
  );
}
function publication(value: unknown, assembly: Record<string, unknown>): boolean {
  if (!isRecord(value) || !isRecord(assembly.job) || !Array.isArray(value.files)) return false;
  const files = value.files;
  return (
    value.assembly_id === assembly.assembly_id &&
    id(value.attempt_id) &&
    value.ready === true &&
    assembly.job.status === 'completed' &&
    value.sample_rate === 48000 &&
    channels(value.channels) &&
    positive(value.frames) &&
    value.frames === assembly.frames &&
    digest(value.pcm_sha256) &&
    integer(value.peak_sample_abs) &&
    value.peak_sample_abs <= 32768 &&
    integer(value.full_scale_samples) &&
    value.full_scale_samples <= Number(value.frames) * Number(value.channels) &&
    value.files.length === (assembly.video ? 4 : 2) &&
    isArrayOf(
      value.files,
      (file) =>
        isRecord(file) &&
        id(file.artifact_id) &&
        (assembly.video
          ? ['wav', 'manifest', 'video', 'video_manifest']
          : ['wav', 'manifest']
        ).includes(String(file.kind)) &&
        digest(file.sha256) &&
        positive(file.size_bytes),
    ) &&
    value.files.filter((file) => isRecord(file) && file.kind === 'wav').length === 1 &&
    value.files.filter((file) => isRecord(file) && file.kind === 'manifest').length === 1 &&
    (!assembly.video ||
      ['video', 'video_manifest'].every(
        (role) => files.filter((file) => isRecord(file) && file.kind === role).length === 1,
      )) &&
    new Set(value.files.map((file) => (isRecord(file) ? file.artifact_id : null))).size ===
      value.files.length
  );
}
export function validateAssemblyView(value: unknown): boolean {
  return (
    isRecord(value) &&
    id(value.assembly_id) &&
    videoSource(value.video) &&
    id(value.source_run_id) &&
    digest(value.plan_sha256) &&
    settings(value.settings) &&
    positive(value.utterance_count) &&
    value.utterance_count <= 8192 &&
    positive(value.frames) &&
    isRecord(value.settings) &&
    Array.isArray(value.settings.pause_after_ms) &&
    value.settings.pause_after_ms.length === value.utterance_count &&
    isNullable(
      value.job,
      (job) => validateJobDto(job) && isRecord(job) && job.kind === 'audioAssembly',
    ) &&
    isNullable(value.result, (result) => publication(result, value))
  );
}
export const audioAssemblyResultValidators = {
  inspect_audio_assembly_cmd: validateAssemblyPreview,
  prepare_audio_assembly_cmd: validateAssemblyView,
  start_audio_assembly_cmd: validateAssemblyView,
  resume_audio_assembly_cmd: validateAssemblyView,
  get_audio_assembly_cmd: validateAssemblyView,
  verify_audio_assembly_cmd: validateAssemblyView,
  list_audio_assemblies_cmd: (value) =>
    isArrayOf(value, validateAssemblyView) && Array.isArray(value) && value.length <= 100,
} satisfies { [K in keyof AudioAssemblyCommandMap]: Validator };
