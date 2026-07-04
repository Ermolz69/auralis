import { ProjectHeader } from '../../../widgets/project-header';
import { TranscriptEditor } from '../../../widgets/transcript-editor';
import { JobQueuePanel } from '../../../widgets/job-queue-panel';
import { ExportPanel } from '../../../widgets/export-panel';
import { Page } from '../../../shared/ui/page-layout';

export const ProjectPage = () => {
  return (
    <Page className="h-screen flex flex-col">
      <ProjectHeader />
      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex flex-col min-w-0">
          <TranscriptEditor />
          <ExportPanel />
        </div>
        <JobQueuePanel />
      </div>
    </Page>
  );
};
