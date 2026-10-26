import fsp from 'node:fs/promises';
import path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { Blocked } from './core.mjs';

const exec = promisify(execFile);
export async function gpuBytes() {
  const { stdout } = await exec(
    'nvidia-smi',
    ['--query-gpu=memory.used', '--format=csv,noheader,nounits'],
    { windowsHide: true, timeout: 5000 },
  );
  if (!stdout.trim()) throw new Error('GPU observer returned no data');
  const values = stdout
    .trim()
    .split(/\r?\n/)
    .map((value) => Number(value.trim()) * 1024 ** 2);
  if (!values.length || values.some((value) => !Number.isSafeInteger(value) || value < 0))
    throw new Error('GPU observer returned invalid data');
  return Math.max(...values);
}

export async function profileBytes(directory) {
  let bytes = 0,
    count = 0;
  const deadline = Date.now() + 120000;
  const pending = [directory];
  while (pending.length) {
    const current = pending.pop();
    if (Date.now() > deadline) throw new Error('Profile disk audit exceeded 120 seconds');
    const entries = await fsp.readdir(current, { withFileTypes: true }).catch((error) => {
      if (error.code === 'ENOENT') return [];
      throw error;
    });
    for (const entry of entries) {
      if (++count > 200000) throw new Error('Profile observer inventory exceeded its bound');
      if (entry.isSymbolicLink()) throw new Error('Profile observer refused redirected data');
      const target = path.join(current, entry.name);
      if (entry.isDirectory()) pending.push(target);
      else if (entry.isFile())
        bytes += await fsp.stat(target).then(
          (s) => s.size,
          (error) => {
            if (error.code === 'ENOENT') return 0;
            throw error;
          },
        );
    }
  }
  return bytes;
}

export function enforceResources(sample, budget, baseline = false) {
  const limits = [
    ['rss_bytes', budget.max_rss_bytes],
    ['vram_bytes', baseline ? budget.baseline_vram_bytes : budget.max_vram_bytes],
    ['profile_bytes', budget.max_profile_bytes],
  ];
  for (const [key, limit] of limits) {
    if (!Number.isSafeInteger(sample[key]) || sample[key] < 0)
      throw new Error(`Resource observer missing ${key}`);
    if (sample[key] > limit)
      throw new Blocked(`Resource budget exceeded: ${key}=${sample[key]} > ${limit}`);
  }
  if (!Number.isSafeInteger(sample.free_disk_bytes))
    throw new Error('Disk observer returned invalid data');
  if (sample.free_disk_bytes < budget.min_free_disk_bytes)
    throw new Blocked('Free disk budget is unavailable');
}

export class ResourceObserver {
  constructor(run, budget, processes, onFailure) {
    Object.assign(this, { run, budget, processes, onFailure });
    this.high_water = { rss_bytes: 0, vram_bytes: 0, profile_bytes: 0 };
    this.samples = 0;
    this.max_interval_ms = 0;
  }
  async sample(baseline = false) {
    try {
      const now = Date.now();
      if (this.lastSample)
        this.max_interval_ms = Math.max(this.max_interval_ms, now - this.lastSample);
      this.lastSample = now;
      const [processes, vram_bytes, disk] = await Promise.all([
        this.processes(),
        gpuBytes(),
        fsp.statfs(this.run.directory),
      ]);
      const profile_bytes = this.profile_bytes ?? 0;
      const rss_bytes = processes.reduce((total, p) => total + p.rss_bytes, 0);
      const sample = {
        observed_at: new Date().toISOString(),
        rss_bytes,
        vram_bytes,
        profile_bytes,
        profile_observed_at: this.profile_observed_at ?? null,
        free_disk_bytes: disk.bavail * disk.bsize,
      };
      this.samples++;
      for (const key of Object.keys(this.high_water))
        this.high_water[key] = Math.max(this.high_water[key], sample[key]);
      await fsp.appendFile(
        path.join(this.run.directory, 'resources.jsonl'),
        JSON.stringify(sample) + '\n',
      );
      enforceResources(sample, this.budget, baseline);
      if (this.max_interval_ms > 10000)
        throw new Error('Resource sampling stalled for more than 10 seconds');
      return sample;
    } catch (error) {
      return this.fail(error, 'fast');
    }
  }
  start() {
    this.timer = setInterval(() => {
      if (!this.pending && !this.failure) {
        this.pending = this.sample()
          .catch(() => {})
          .finally(() => {
            this.pending = null;
          });
      }
    }, this.budget.sample_interval_ms);
    this.diskTimer = setInterval(() => {
      if (!this.diskPending && !this.failure)
        this.diskPending = this.auditProfile()
          .catch(() => {})
          .finally(() => {
            this.diskPending = null;
          });
    }, this.budget.disk_audit_interval_ms);
  }
  async auditProfile() {
    const start = Date.now();
    try {
      this.profile_bytes = await profileBytes(path.join(this.run.directory, 'profile'));
      this.profile_observed_at = new Date().toISOString();
      this.high_water.profile_bytes = Math.max(this.high_water.profile_bytes, this.profile_bytes);
      await fsp.appendFile(
        path.join(this.run.directory, 'resources.jsonl'),
        JSON.stringify({
          profile_observed_at: this.profile_observed_at,
          profile_bytes: this.profile_bytes,
          profile_audit_duration_ms: Date.now() - start,
        }) + '\n',
      );
      if (this.profile_bytes > this.budget.max_profile_bytes)
        throw new Blocked('Owned profile disk budget exceeded');
    } catch (error) {
      return this.fail(error, 'profile-disk');
    }
  }
  async fail(error, observer) {
    this.failure ??= error;
    try {
      if (this.onFailure) await this.onFailure(this.failure);
    } catch (stopError) {
      this.stop_error = stopError.message;
    }
    try {
      await fsp.appendFile(
        path.join(this.run.directory, 'resources.jsonl'),
        JSON.stringify({
          observed_at: new Date().toISOString(),
          observer_error: this.failure.message,
          observer,
        }) + '\n',
      );
    } catch (retentionError) {
      this.retention_error = retentionError.message;
    }
    throw this.failure;
  }
  check() {
    if (this.failure) throw this.failure;
  }
  async stop() {
    clearInterval(this.timer);
    clearInterval(this.diskTimer);
    await Promise.all([this.pending, this.diskPending]);
    return {
      samples: this.samples,
      high_water: this.high_water,
      max_interval_ms: this.max_interval_ms,
      observer_error: this.failure?.message ?? null,
      retention_error: this.retention_error ?? null,
      stop_error: this.stop_error ?? null,
    };
  }
}
