import { PasteYoutubeLink } from '../../../features/paste-youtube-link';
import { Page, PageContainer, PageContent } from '../../../shared/ui/page-layout';

export const HomePage = () => {
  return (
    <Page className="flex flex-col items-center justify-center">
      <PageContainer size="sm" className="text-center justify-center items-center">
        <PageContent className="items-center justify-center gap-8">
          <h1 className="text-5xl font-bold bg-gradient-to-r from-primary to-danger bg-clip-text text-transparent pb-2">
            Auralis
          </h1>
          <p className="text-muted text-xl">AI-powered video dubbing straight from your desktop.</p>
          <div className="mt-4 w-full">
            <PasteYoutubeLink />
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  );
};
