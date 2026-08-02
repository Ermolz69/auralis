import { ProjectHeader } from '../../../widgets/project-header';
import { Page } from '../../../shared/ui/page-layout';
import { WorkspaceMain } from './WorkspaceMain';
import { WorkspaceSecondaryPanels } from './WorkspaceSecondaryPanels';

export const ProjectPage = () => {
  return (
    <Page className="h-screen flex flex-col overflow-hidden">
      <ProjectHeader />
      <main
        className="flex-1 min-h-0 overflow-hidden"
        data-testid="project-workspace"
        aria-label="Project workspace"
      >
        <div className="flex h-full min-h-0 flex-col overflow-hidden xl:grid xl:grid-cols-[minmax(0,1fr)_auto_auto]">
          <WorkspaceMain />
          <WorkspaceSecondaryPanels />
        </div>
      </main>
    </Page>
  );
};
