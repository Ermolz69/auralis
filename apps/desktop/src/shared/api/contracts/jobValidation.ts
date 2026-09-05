import type { Job, JobEvent } from './job';

export function isSafeRevision(revision: unknown): boolean {
  return (
    typeof revision === 'number' &&
    Number.isSafeInteger(revision) &&
    revision >= 1 &&
    revision <= Number.MAX_SAFE_INTEGER
  );
}

const VALID_STATUSES = new Set(['pending', 'running', 'completed', 'failed', 'cancelled']);

export function validateJobDto(job: unknown): job is Job {
  if (!job || typeof job !== 'object') return false;
  const candidate = job as Record<string, unknown>;

  if (typeof candidate.id !== 'string') return false;
  if (typeof candidate.kind !== 'string' || candidate.kind.trim().length === 0) return false;
  if (!isSafeRevision(candidate.revision)) return false;
  if (candidate.projectId !== null && typeof candidate.projectId !== 'string') return false;
  if (typeof candidate.title !== 'string') return false;
  if (typeof candidate.status !== 'string' || !VALID_STATUSES.has(candidate.status)) return false;
  if (candidate.stage !== null && typeof candidate.stage !== 'string') return false;

  if (!candidate.progress || typeof candidate.progress !== 'object') return false;
  const progress = candidate.progress as Record<string, unknown>;
  if (typeof progress.percent !== 'number') return false;
  if (typeof progress.message !== 'string') return false;
  if (progress.currentStep !== null && typeof progress.currentStep !== 'string') return false;
  if (progress.processedItems !== null && typeof progress.processedItems !== 'number') return false;
  if (progress.totalItems !== null && typeof progress.totalItems !== 'number') return false;

  if (candidate.error !== null && typeof candidate.error !== 'string') return false;
  if (typeof candidate.createdAt !== 'string') return false;
  if (typeof candidate.updatedAt !== 'string') return false;

  return true;
}

const VALID_EVENT_KINDS = new Set([
  'created',
  'started',
  'progressed',
  'completed',
  'failed',
  'cancelled',
]);

export function validateJobEventDto(event: unknown): event is JobEvent {
  if (!event || typeof event !== 'object') return false;
  const candidate = event as Record<string, unknown>;

  return (
    typeof candidate.kind === 'string' &&
    VALID_EVENT_KINDS.has(candidate.kind) &&
    validateJobDto(candidate.job)
  );
}

export function validateJobSnapshot(snapshot: unknown): snapshot is Job[] {
  if (!Array.isArray(snapshot)) return false;

  const ids = new Set<string>();
  for (const job of snapshot) {
    if (!validateJobDto(job) || ids.has(job.id)) return false;
    ids.add(job.id);
  }
  return true;
}
