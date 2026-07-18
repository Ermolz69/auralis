import type { JobDto, JobEventDto } from './types';

export function isSafeRevision(revision: unknown): boolean {
  return typeof revision === 'number' && Number.isSafeInteger(revision) && revision >= 1 && revision <= 9007199254740991;
}

const VALID_STATUSES = new Set(['pending', 'running', 'completed', 'failed', 'cancelled']);

export function validateJobDto(job: unknown): job is JobDto {
  if (!job || typeof job !== 'object') return false;
  const j = job as Record<string, unknown>;
  
  if (typeof j.id !== 'string') return false;
  if (!isSafeRevision(j.revision)) return false;
  if (j.projectId !== null && typeof j.projectId !== 'string') return false;
  if (typeof j.title !== 'string') return false;
  if (typeof j.status !== 'string' || !VALID_STATUSES.has(j.status)) return false;
  if (j.stage !== null && typeof j.stage !== 'string') return false;
  
  if (!j.progress || typeof j.progress !== 'object') return false;
  const p = j.progress as Record<string, unknown>;
  if (typeof p.percent !== 'number') return false;
  if (typeof p.message !== 'string') return false;
  if (p.currentStep !== null && typeof p.currentStep !== 'string') return false;
  if (p.processedItems !== null && typeof p.processedItems !== 'number') return false;
  if (p.totalItems !== null && typeof p.totalItems !== 'number') return false;

  if (j.error !== null && typeof j.error !== 'string') return false;
  if (typeof j.createdAt !== 'string') return false;
  if (typeof j.updatedAt !== 'string') return false;
  
  return true;
}

const VALID_EVENT_KINDS = new Set(['created', 'started', 'progressed', 'completed', 'failed', 'cancelled']);

export function validateJobEventDto(event: unknown): event is JobEventDto {
  if (!event || typeof event !== 'object') return false;
  const e = event as Record<string, unknown>;
  
  if (typeof e.kind !== 'string' || !VALID_EVENT_KINDS.has(e.kind)) return false;
  if (!validateJobDto(e.job)) return false;
  
  return true;
}

export function validateJobSnapshot(snapshot: unknown, expectedProjectId: string | null): snapshot is JobDto[] {
  if (!Array.isArray(snapshot)) return false;
  
  const ids = new Set<string>();
  for (const job of snapshot) {
    if (!validateJobDto(job)) return false;
    if (job.projectId !== expectedProjectId) return false;
    if (ids.has(job.id)) return false;
    ids.add(job.id);
  }
  
  return true;
}
