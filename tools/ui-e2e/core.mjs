import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const root = fileURLToPath(new URL('../../', import.meta.url));
export const storage = path.join(root, '.cache/ui-e2e');
export const runs = path.join(storage, 'runs');
export class Blocked extends Error {}
export class UnconfirmedCleanup extends Error {}
export const idPattern = /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,120}$/;
export function owned(parent, target) {
  const relative = path.relative(path.resolve(parent), path.resolve(target));
  if (
    !relative ||
    relative === '..' ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  )
    throw new Error('Expected a child of the owned test directory');
  return path.resolve(target);
}
export async function json(file, maxBytes = 8 * 1024 * 1024) {
  const stat = await fsp.lstat(file);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > maxBytes)
    throw new Error('Expected a bounded regular JSON file');
  return JSON.parse(await fsp.readFile(file, 'utf8'));
}
export async function hash(file, check = () => {}, signal) {
  const stat = await fsp.lstat(file);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('Expected a regular file');
  const digest = crypto.createHash('sha256');
  for await (const block of fs.createReadStream(file, { signal })) {
    check();
    digest.update(block);
  }
  return digest.digest('hex');
}
export async function uniqueRun(layer) {
  await fsp.mkdir(runs, { recursive: true });
  const id = `${new Date().toISOString().replace(/[:.]/g, '-')}-${layer}-${crypto.randomUUID().slice(0, 8)}`;
  const directory = owned(runs, path.join(runs, id));
  await fsp.mkdir(directory);
  return { id, directory };
}
export function overall(steps) {
  if (steps.some((step) => step.status === 'failed')) return 'failed';
  if (steps.some((step) => step.status === 'blocked')) return 'blocked';
  if (!steps.length || steps.some((step) => step.status === 'not_run')) return 'not_run';
  return 'passed';
}
export function validateSpec(spec) {
  if (
    spec.tts_execution_budget_seconds !== undefined &&
    (!Number.isSafeInteger(spec.tts_execution_budget_seconds) ||
      spec.tts_execution_budget_seconds < 1 ||
      spec.tts_execution_budget_seconds > 172800)
  )
    throw new Error('Invalid finite TTS execution budget');
  for (const flag of ['allow_unverified_neutral', 'install_translation', 'export_video'])
    if (spec[flag] !== undefined && typeof spec[flag] !== 'boolean')
      throw new Error(`Invalid ${flag} policy`);
  if (spec.max_attempts !== undefined && spec.max_attempts !== 1)
    throw new Error('Real cases permit one attempt');
  if (spec.max_source_media_bytes !== undefined && spec.max_source_media_bytes !== 1073741824)
    throw new Error('This source case has a fixed 1 GiB byte budget');
  if (
    spec.allow_inference === true &&
    (!spec.runtime_seed ||
      !Array.isArray(spec.runtime_seed.files) ||
      !spec.runtime_seed.files.length)
  )
    throw new Error('Inference requires a frozen runtime seed manifest');
  if (spec.version !== 1 || !idPattern.test(spec.case_id ?? ''))
    throw new Error('Invalid case identity');
  if (!/^[a-f0-9]{64}$/.test(spec.candidate_sha256 ?? ''))
    throw new Error('Pin the candidate SHA-256');
  if (!/^[a-f0-9]{64}$/.test(spec.source_subtitle_sha256 ?? ''))
    throw new Error('Pin the Chinese subtitle SHA-256');
  if (spec.source_url !== 'https://www.youtube.com/watch?v=HRaa8kb1EhE')
    throw new Error('Unexpected frozen source');
  if (
    spec.track?.language !== 'zh' ||
    spec.track?.format !== 'srt' ||
    spec.track?.automatic !== false
  )
    throw new Error('Select the frozen manual Chinese SRT track');
  if (
    !Number.isInteger(spec.max_wall_seconds) ||
    spec.max_wall_seconds < 60 ||
    spec.max_wall_seconds > 604800
  )
    throw new Error('Invalid finite case deadline');
  if (spec.exercise_cancellation !== undefined && typeof spec.exercise_cancellation !== 'boolean')
    throw new Error('Invalid cancellation policy');
  if (spec.allow_inference === true) {
    if (spec.tts_execution_budget_seconds === undefined)
      throw new Error('Pin the host TTS execution budget');
    if (!/^[a-f0-9]{64}$/.test(spec.candidate_receipt_sha256 ?? ''))
      throw new Error('Pin the candidate build receipt SHA-256');
    if (!['diagnostic', 'full-source'].includes(spec.case_scope))
      throw new Error('Pin inference case_scope');
    if (
      !Number.isSafeInteger(spec.expected_cue_count) ||
      spec.expected_cue_count < 1 ||
      spec.expected_cue_count > 100000
    )
      throw new Error('Pin expected_cue_count');
    if (
      spec.case_scope === 'full-source' &&
      (spec.expected_cue_count !== 4004 ||
        spec.export_video !== true ||
        spec.install_translation !== true ||
        spec.exercise_cancellation === true)
    )
      throw new Error('Full source requires all 4004 cues, video and one start per engine');
    if (!/^[a-f0-9]{64}$/.test(spec.source_timing_sha256 ?? ''))
      throw new Error('Pin source_timing_sha256');
    for (const key of ['source_duration_ms', 'source_duration_tolerance_ms'])
      if (!Number.isSafeInteger(spec[key]) || spec[key] <= 0) throw new Error(`Pin ${key}`);
    if (spec.source_duration_tolerance_ms > 2000)
      throw new Error('Source video tolerance exceeds 2 seconds');
    const b = spec.resource_budget;
    if (!b || b.sample_interval_ms !== 1000) throw new Error('Pin 1000 ms resource sampling');
    if (b.disk_audit_interval_ms !== 30000)
      throw new Error('Pin 30000 ms full profile disk auditing');
    const maximums = {
      max_rss_bytes: 24 * 1024 ** 3,
      max_vram_bytes: 7680 * 1024 ** 2,
      baseline_vram_bytes: 2048 * 1024 ** 2,
      max_profile_bytes: 64 * 1024 ** 3,
    };
    for (const [key, limit] of Object.entries(maximums))
      if (!Number.isSafeInteger(b[key]) || b[key] <= 0 || b[key] > limit)
        throw new Error(`Invalid resource ${key}`);
    if (!Number.isSafeInteger(b.min_free_disk_bytes) || b.min_free_disk_bytes < 64 * 1024 ** 3)
      throw new Error('Reserve at least 64 GiB free disk');
    if (
      spec.case_scope === 'diagnostic' &&
      (typeof spec.diagnostic_text !== 'string' ||
        !spec.diagnostic_text.trim() ||
        spec.diagnostic_text.length > 1000 ||
        spec.export_video === true ||
        spec.exercise_cancellation === true)
    )
      throw new Error('Pin bounded diagnostic text without video or cancellation');
  }
  if (
    !Array.isArray(spec.engines) ||
    spec.engines.length !== 2 ||
    spec.engines[0].engine_id !== 'voxcpm2' ||
    spec.engines[1].engine_id !== 'espeech'
  )
    throw new Error('Pin sequential VoxCPM2 and ESpeech selections');
  for (const selection of spec.engines)
    for (const key of ['profile_id', 'voice_id', 'reference_id'])
      if (typeof selection[key] !== 'string' || !selection[key]) throw new Error(`Pin ${key}`);
  if (spec.allow_inference === true) {
    if (
      !Array.isArray(spec.engine_identities) ||
      spec.engine_identities.length !== spec.engines.length
    )
      throw new Error('Pin exact engine_identities');
    for (let i = 0; i < spec.engines.length; i++) {
      const pin = spec.engine_identities[i];
      for (const key of ['engine_id', 'profile_id', 'voice_id', 'reference_id'])
        if (pin[key] !== spec.engines[i][key]) throw new Error('Engine identity selection differs');
      for (const key of ['profile_sha256', 'voice_manifest_sha256', 'reference_audio_sha256'])
        if (!/^[a-f0-9]{64}$/.test(pin[key] ?? '')) throw new Error(`Pin ${key}`);
      if (!/^[a-f0-9]{40}$/.test(pin.model_revision ?? ''))
        throw new Error('Pin exact model_revision');
    }
  }
  return spec;
}
