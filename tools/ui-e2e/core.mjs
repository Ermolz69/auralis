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
export async function json(file) {
  const stat = await fsp.lstat(file);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 8 * 1024 * 1024)
    throw new Error('Expected a bounded regular JSON file');
  return JSON.parse(await fsp.readFile(file, 'utf8'));
}
export async function hash(file) {
  const stat = await fsp.lstat(file);
  if (!stat.isFile() || stat.isSymbolicLink()) throw new Error('Expected a regular file');
  const digest = crypto.createHash('sha256');
  for await (const block of fs.createReadStream(file)) digest.update(block);
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
    spec.max_wall_seconds > 28800
  )
    throw new Error('Invalid finite case deadline');
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
  return spec;
}
