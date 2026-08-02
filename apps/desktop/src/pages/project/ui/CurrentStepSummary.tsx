import { useContext } from 'react';
import {
  formatJobStatus,
  getJobStatusTone,
  isActiveJobStatus,
  useJobContext,
} from '@/entities/job';
import { ProjectContext } from '@/entities/project';
import { formatProjectTitle } from '@/entities/media';
import { Badge } from '@/shared/ui/badge';
import { Progress } from '@/shared/ui/progress';

export function CurrentStepSummary() {
  const { activeJobs, phase, pendingRefetch, scopeProjectId } = useJobContext();
  const project = useContext(ProjectContext)?.project ?? null;
  const activeJob = activeJobs.find((job) => isActiveJobStatus(job.status));

  if (!scopeProjectId) return null;

  if (!activeJob) {
    return (
      <section
        className="shrink-0 border-b border-muted bg-surface px-6 py-3"
        aria-label="Current step"
      >
        <div className="flex flex-col gap-1">
          <h2 className="text-sm font-semibold text-text">Current step</h2>
          <p className="text-sm text-muted">No operation is running for this project.</p>
        </div>
      </section>
    );
  }

  const statusLabel = formatJobStatus(activeJob);
  const message = activeJob.progress.message || 'Working';
  const backendPercent = Number.isFinite(activeJob.progress.percent)
    ? activeJob.progress.percent
    : null;
  const isIndeterminate =
    backendPercent === null || (activeJob.status === 'pending' && backendPercent === 0);
  const projectLabel =
    project && activeJob.projectId === project.id
      ? formatProjectTitle(project.title, project.source)
      : 'Current project';
  const freshnessLabel =
    phase === 'initializing'
      ? 'Connecting job updates'
      : phase === 'synchronizing'
        ? 'Refreshing latest job state'
        : phase === 'stale' || pendingRefetch
          ? 'Job state is refreshing'
          : null;

  return (
    <section
      className="shrink-0 border-b border-muted bg-surface px-6 py-3"
      aria-label="Current step"
      role={phase === 'stale' || pendingRefetch ? 'alert' : 'status'}
      aria-live={phase === 'stale' || pendingRefetch ? 'assertive' : 'polite'}
    >
      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="min-w-0">
            <h2 className="text-sm font-semibold text-text">Current step</h2>
            <p className="truncate text-xs text-muted">Project: {projectLabel}</p>
            <p className="truncate text-sm text-muted">{statusLabel}</p>
          </div>
          {backendPercent !== null && (
            <Badge variant={getJobStatusTone(activeJob.status)} size="sm">
              {backendPercent}%
            </Badge>
          )}
        </div>
        <Progress
          value={backendPercent ?? 0}
          variant={getJobStatusTone(activeJob.status)}
          indeterminate={isIndeterminate}
          label={`${activeJob.title} current progress`}
        />
        <p className="truncate text-xs text-muted">{message}</p>
        {freshnessLabel && <p className="truncate text-xs text-warning">{freshnessLabel}</p>}
      </div>
    </section>
  );
}
