import { ProjectHeader } from '../../../widgets/project-header';
import { TranscriptEditor } from '../../../widgets/transcript-editor';
import { JobQueuePanel } from '../../../widgets/job-queue-panel';
import { ExportPanel } from '../../../widgets/export-panel';
import { MediaPanel } from '../../../widgets/media-panel';
import { Page } from '../../../shared/ui/page-layout';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../../shared/ui/tabs';

export const ProjectPage = () => {
  return (
    <Page className="h-screen flex flex-col overflow-hidden">
      <ProjectHeader />
      <div className="flex-1 min-h-0 overflow-hidden" data-testid="project-workspace">
        <div
          className="hidden xl:grid h-full min-h-0 overflow-hidden grid-cols-[minmax(16rem,20rem)_minmax(0,1fr)_minmax(18rem,24rem)]"
          data-testid="workspace-wide"
        >
          <MediaPanel className="border-r border-muted" />
          <WorkspaceMain />
          <JobQueuePanel className="border-l border-muted" />
        </div>
        <div
          className="xl:hidden flex h-full min-h-0 flex-col overflow-hidden"
          data-testid="workspace-compact"
        >
          <WorkspaceMain />
          <WorkspaceSecondaryTabs />
        </div>
      </div>
    </Page>
  );
};

function WorkspaceMain() {
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <TranscriptEditor />
      <ExportPanel />
    </div>
  );
}

function WorkspaceSecondaryTabs() {
  return (
    <Tabs defaultValue="media" variant="compact" fullWidth className="shrink-0 border-t border-muted bg-bg p-3">
      <TabsList className="w-full">
        <TabsTrigger value="media">Media</TabsTrigger>
        <TabsTrigger value="jobs">Jobs</TabsTrigger>
      </TabsList>
      <TabsContent value="media" className="max-h-[38vh] overflow-hidden">
        <MediaPanel className="rounded-md border border-muted" />
      </TabsContent>
      <TabsContent value="jobs" className="max-h-[38vh] overflow-hidden">
        <JobQueuePanel className="rounded-md border border-muted" />
      </TabsContent>
    </Tabs>
  );
}
