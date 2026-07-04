import { Card, CardContent } from '../../../shared/ui/card';
import { Progress } from '../../../shared/ui/progress';
import { Icon } from '../../../shared/ui/icon';

export const JobQueuePanel = () => {
  const jobs: { title: string; progress: number }[] = []; // Simulating empty state

  return (
    <aside className="w-full h-full bg-surface border-l border-muted p-6 flex flex-col gap-4 overflow-hidden">
      <h2 className="text-lg font-semibold text-text shrink-0">Job Queue</h2>
      <div className="flex-1 flex flex-col gap-3 overflow-y-auto min-h-0">
        {jobs.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center text-center p-4">
            <Icon name="Inbox" size="lg" className="text-muted/50 mb-3" />
            <p className="text-text font-medium">Queue is empty</p>
            <p className="text-sm text-muted mt-1">Exported jobs will appear here</p>
          </div>
        ) : (
          jobs.map((job, idx) => (
            <Card key={idx} variant="muted" className="shrink-0">
              <CardContent className="p-4 flex flex-col gap-3">
                <p className="text-sm text-text font-medium">{job.title}</p>
                <Progress value={job.progress} />
              </CardContent>
            </Card>
          ))
        )}
      </div>
    </aside>
  );
};
