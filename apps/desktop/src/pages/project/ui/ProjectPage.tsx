import { ProjectHeader } from '../../../widgets/project-header';
import { TranscriptEditor } from '../../../widgets/transcript-editor';
import { JobQueuePanel } from '../../../widgets/job-queue-panel';
import { ExportPanel } from '../../../widgets/export-panel';

export const ProjectPage = () => {
  return (
    <div className="h-screen flex flex-col bg-bg text-text font-sans">
      <ProjectHeader />
      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex flex-col min-w-0">
          <TranscriptEditor />
          <ExportPanel />
        </div>
        <JobQueuePanel />
      </div>
    </div>
  );
};
