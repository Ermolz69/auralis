import { ProjectList } from '../../../features/project-list';
import { PasteYoutubeLink } from '@/features/paste-youtube-link';
import { ImportLocalMediaButton } from '@/features/import-local-media';
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
          <section
            className="mb-5 space-y-4 rounded-md border border-border bg-surface p-4"
            aria-label="Create project"
          >
            <h2 className="text-sm font-semibold text-text">Add a source</h2>
            <ImportLocalMediaButton newProject />
            <PasteYoutubeLink newProject />
          </section>
          <ProjectList />
        </div>
      </div>
    </Page>
  );
};
