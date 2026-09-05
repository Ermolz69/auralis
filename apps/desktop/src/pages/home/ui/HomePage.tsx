import { ProjectList } from '../../../features/project-list';
import {
  Page,
  PageDescription,
  PageHeaderGroup,
  PageTitle,
  PageTopBar,
} from '../../../shared/ui/page-layout';

export const HomePage = () => {
  return (
    <Page className="flex h-full min-h-0 animate-content-in flex-col overflow-hidden">
      <PageTopBar>
        <PageHeaderGroup className="!gap-0">
          <PageTitle className="!text-xl">Projects</PageTitle>
          <PageDescription className="mt-0.5 !text-xs">
            Local media workspaces on this device
          </PageDescription>
        </PageHeaderGroup>
      </PageTopBar>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6 lg:px-8">
        <div className="mx-auto w-full max-w-5xl">
          <ProjectList />
        </div>
      </div>
    </Page>
  );
};
