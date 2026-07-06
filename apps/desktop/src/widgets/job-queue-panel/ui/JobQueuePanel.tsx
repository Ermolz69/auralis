import { useEffect, useState } from 'react';
import { Card, CardContent } from '../../../shared/ui/card';
import { Progress } from '../../../shared/ui/progress';
import { Icon } from '../../../shared/ui/icon';
import { listJobs, subscribeJobEvents } from '@/entities/job';
import type { Job } from '@/entities/job';
import { CancelJobButton } from '@/features/cancel-job';

export const JobQueuePanel = () => {
  const [jobs, setJobs] = useState<Job[]>([]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      try {
        const initialJobs = await listJobs();
        setJobs(initialJobs);

        unlisten = await subscribeJobEvents((event) => {
          setJobs((current) => {
            const index = current.findIndex((j) => j.id === event.jobId);
            if (index >= 0) {
              const newJobs = [...current];
              newJobs[index] = {
                ...newJobs[index],
                status: event.status,
                stage: event.stage,
                progress: event.progress,
                error: event.error || newJobs[index].error,
              };
              return newJobs;
            } else {
              // If new job, fetch all to get full details like title
              listJobs().then(setJobs).catch(console.error);
              return current;
            }
          });
        });
      } catch (e) {
        console.error('Failed to setup JobQueuePanel', e);
      }
    };

    setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  return (
    <aside className="w-full h-full bg-surface border-l border-muted p-6 flex flex-col gap-4 overflow-hidden">
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
                      {job.status.replace('_', ' ')} {job.stage && `- ${job.stage.replace(/_/g, ' ')}`}
                    </p>
                  </div>
                  {(job.status === 'queued' || job.status === 'running') && (
                    <CancelJobButton jobId={job.id} />
                  )}
                </div>
                
                {job.status === 'failed' && job.error ? (
                  <p className="text-xs text-red-400">{job.error}</p>
                ) : (
                  <Progress value={job.progress.percent} />
                )}
              </CardContent>
            </Card>
          ))
        )}
      </div>
    </aside>
  );
};
