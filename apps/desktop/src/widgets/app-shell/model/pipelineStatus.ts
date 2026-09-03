import { isActiveJobStatus, type JobDto, type JobKind, type JobStatus } from '@/entities/job';
import type { PipelineStep } from '@/shared/router';

export type PipelineDisplayStatus = JobStatus | 'idle' | 'unavailable';

const JOB_KINDS_BY_STEP = {
  // Source imports currently finish synchronously and expose their result on the project.
  source: [],
  // The current backend dubbing runner imports YouTube subtitles.
  subtitles: ['dubbing'],
} satisfies Record<PipelineStep, readonly JobKind[]>;

function getStepJobStatus(jobs: readonly JobDto[], kinds: readonly JobKind[]): JobStatus | null {
  let selected: JobDto | undefined;
  for (const job of jobs) {
    if (!kinds.includes(job.kind)) continue;
    if (!selected) {
      selected = job;
      continue;
    }
    const active = isActiveJobStatus(job.status);
    const selectedActive = isActiveJobStatus(selected.status);
    if (active !== selectedActive) {
      if (active) selected = job;
      continue;
    }
    // Attempts are ordered by creation, not progress updates or snapshot insertion order.
    const createdAt = Date.parse(job.createdAt) || 0;
    const selectedCreatedAt = Date.parse(selected.createdAt) || 0;
    if (
      createdAt > selectedCreatedAt ||
      (createdAt === selectedCreatedAt && job.id > selected.id)
    ) {
      selected = job;
    }
  }
  return selected?.status ?? null;
}

export function getPipelineStatus(hasSource: boolean, jobs: readonly JobDto[]) {
  return {
    source: getStepJobStatus(jobs, JOB_KINDS_BY_STEP.source) ?? (hasSource ? 'completed' : 'idle'),
    subtitles: getStepJobStatus(jobs, JOB_KINDS_BY_STEP.subtitles) ?? 'idle',
  } satisfies Record<PipelineStep, PipelineDisplayStatus>;
}
