import { useEffect, useState } from 'react';
import { Card, CardContent } from '../../../shared/ui/card';
import { Progress } from '../../../shared/ui/progress';
import { Icon } from '../../../shared/ui/icon';
import { listJobs, subscribeJobEvents, subscribeJobsInvalidated } from '@/entities/job';
import type { Job } from '@/entities/job';
import { CancelJobButton } from '@/features/cancel-job';
import { useProjectContext } from '@/entities/project';

const formatStage = (stage: string | null) => {
  if (!stage) return '';
  const withSpaces = stage.replace(/([A-Z])/g, ' $1');
  return withSpaces.charAt(0).toUpperCase() + withSpaces.slice(1);
};

export const JobQueuePanel = () => {
  const [jobs, setJobs] = useState<Job[]>([]);
  const { projectId } = useProjectContext();

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      try {
        const initialJobs = await listJobs();
        if (cancelled) return;
        setJobs(initialJobs.filter((job) => job.projectId === projectId));

        const fn = await subscribeJobEvents((event) => {
          if (event.projectId !== projectId) return;

          setJobs((current) => {
            const index = current.findIndex((j) => j.id === event.jobId);
            if (index >= 0) {
              const newJobs = [...current];
              newJobs[index] = {
                ...newJobs[index],
                status: event.status,
                stage: event.stage,
                progress: event.progress,
                error: event.error ?? newJobs[index].error,
              };
              return newJobs;
            } else {
              // If new job, fetch all to get full details like title
              listJobs()
                .then((allJobs) => setJobs(allJobs.filter((j) => j.projectId === projectId)))
                .catch(console.error);
              return current;
            }
          });
        });

        const invalidatedFn = await subscribeJobsInvalidated(() => {
          console.warn('Job events invalidated (lagged), refetching jobs');
          listJobs()
            .then((allJobs) => setJobs(allJobs.filter((j) => j.projectId === projectId)))
            .catch(console.error);
        });

        if (cancelled) {
          fn();
          invalidatedFn();
        } else {
          unlisten = () => {
            fn();
            invalidatedFn();
          };
        }
      } catch (e) {
        console.error('Failed to setup JobQueuePanel', e);
      }
    };

    setup();

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, [projectId]);

  return (
    <aside className="w-96 shrink-0 h-full bg-surface border-l border-muted p-6 flex flex-col gap-4 overflow-hidden">
      <h2 className="text-lg font-semibold text-text shrink-0">Job Queue</h2>
      <div className="flex-1 flex flex-col gap-3 overflow-y-auto min-h-0">
        {jobs.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center text-center p-4">
            <Icon name="Inbox" size="lg" className="text-muted/50 mb-3" />
            <p className="text-text font-medium">Queue is empty</p>
            <p className="text-sm text-muted mt-1">Jobs will appear here</p>
          </div>
        ) : (
          jobs.map((job) => (
            <Card key={job.id} variant="muted" className="shrink-0">
              <CardContent className="p-4 flex flex-col gap-3">
                <div className="flex justify-between items-start gap-2">
                  <div className="flex flex-col">
                    <p className="text-sm text-text font-medium">{job.title}</p>
                    <p className="text-xs text-muted capitalize">
                      {job.status.replace('_', ' ')}
                      {job.stage && ` - ${formatStage(job.stage)}`}
                    </p>
                  </div>
                  {(job.status === 'pending' || job.status === 'running') && (
                    <CancelJobButton jobId={job.id} />
                  )}
                </div>

                {job.status === 'failed' && job.error ? (
                  <div className="flex flex-col gap-1">
                    <p className="text-xs text-danger">{job.error}</p>
                    {job.progress.message && (
                      <p className="text-xs text-muted">Final state: {job.progress.message}</p>
                    )}
                  </div>
                ) : (
                  <div className="flex flex-col gap-1">
                    <Progress value={job.progress.percent} />
                    <div className="flex justify-between text-xs text-muted">
                      <span>{job.progress.message || 'Initializing...'}</span>
                      <span>{job.progress.percent}%</span>
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          ))
        )}
      </div>
    </aside>
  );
};
