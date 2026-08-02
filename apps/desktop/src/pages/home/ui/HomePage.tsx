import { PasteYoutubeLink } from '../../../features/paste-youtube-link';
import { ProjectList } from '../../../features/project-list';
import { ImportLocalMediaButton } from '../../../features/import-local-media';
import { Page, PageContainer, PageContent } from '../../../shared/ui/page-layout';

export const HomePage = () => {
  return (
    <Page className="flex flex-col items-center justify-center overflow-y-auto">
      <PageContainer size="sm" className="text-center justify-center items-center py-10">
        <PageContent className="items-center justify-center gap-7">
          <h1 className="text-5xl font-bold bg-gradient-to-r from-primary to-danger bg-clip-text text-transparent pb-2">
            Auralis
          </h1>
          <p className="text-muted text-xl">
            Import a local video and keep the work on your desktop.
          </p>
          <div className="mt-2 flex flex-col gap-4 w-full" aria-label="Create project">
            <div className="flex flex-col gap-2">
              <ImportLocalMediaButton />
              <p className="text-left text-sm text-muted">
                Creates a local project, imports metadata, and opens the workspace when ready.
              </p>
            </div>
            <div className="flex items-center gap-4 w-full">
              <hr className="flex-1 border-muted" />
              <span className="text-muted text-sm font-medium uppercase tracking-widest">
                or add a YouTube source
              </span>
              <hr className="flex-1 border-muted" />
            </div>
            <PasteYoutubeLink />
            <ProjectList />
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  );
};
