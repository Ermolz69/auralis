import { useState } from 'react';
import { ImportLocalMediaButton } from '../../../features/import-local-media';
import { PasteYoutubeLink } from '../../../features/paste-youtube-link';
import { ProjectList } from '../../../features/project-list';
import { Button } from '../../../shared/ui/button';
import { Icon } from '../../../shared/ui/icon';
import { Page } from '../../../shared/ui/page-layout';

export const HomePage = () => {
  const [creationOpen, setCreationOpen] = useState(false);

  return (
    <Page className="flex h-full min-h-0 flex-col overflow-hidden">
      <header className="flex shrink-0 items-start justify-between gap-4 border-b border-border px-5 py-5 sm:px-8">
        <div>
          <h1 className="text-xl font-semibold text-text">Projects</h1>
          <p className="mt-0.5 text-xs text-muted">Local media workspaces on this device</p>
        </div>
        <Button
          type="button"
          variant={creationOpen ? 'secondary' : 'primary'}
          size="md"
          aria-expanded={creationOpen}
          aria-controls="create-project-panel"
          onClick={() => setCreationOpen((open) => !open)}
          leftIcon={<Icon name={creationOpen ? 'X' : 'Plus'} size={14} />}
        >
          {creationOpen ? 'Close' : 'New project'}
        </Button>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6 lg:px-8">
        <div className="mx-auto w-full max-w-5xl">
          {creationOpen && (
            <section
              id="create-project-panel"
              aria-label="Create project"
              className="mb-6 rounded-lg border border-border bg-surface p-4 shadow-sm sm:p-5"
            >
              <div className="mb-4">
                <h2 className="text-sm font-semibold text-text">Create a project</h2>
                <p className="mt-1 text-xs text-muted">
                  Choose a local video or attach one supported YouTube source.
                </p>
              </div>
              <div className="grid gap-4 xl:grid-cols-[minmax(14rem,0.7fr)_minmax(22rem,1.3fr)] xl:items-start">
                <div className="rounded-md border border-border bg-surface-raised p-4">
                  <ImportLocalMediaButton />
                  <p className="mt-2 text-xs text-muted">
                    Metadata is inspected locally before the workspace opens.
                  </p>
                </div>
                <div className="rounded-md border border-border bg-surface-raised p-4">
                  <PasteYoutubeLink />
                </div>
              </div>
            </section>
          )}

          <ProjectList />
        </div>
      </div>
    </Page>
  );
};
