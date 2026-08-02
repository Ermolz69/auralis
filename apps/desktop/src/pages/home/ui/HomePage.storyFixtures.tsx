import type { ReactNode } from 'react';
import { Button } from '@/shared/ui/button';
import { Input } from '@/shared/ui/input';
import { Page, PageContainer, PageContent } from '@/shared/ui/page-layout';
import { ProjectListEmptyState } from '@/features/project-list';
import type { HomeProject } from './HomePage.storyData';

export type HomeScenarioProps = {
  list?: ReactNode;
  importStatus?: string;
  importBusy?: boolean;
};

export function HomeScenario({
  list = <ProjectListEmptyState />,
  importStatus,
  importBusy = false,
}: HomeScenarioProps) {
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
          <div className="mt-2 flex w-full flex-col gap-4" aria-label="Create project">
            <div className="flex flex-col gap-2">
              <Button type="button" size="lg" fullWidth loading={importBusy} disabled={importBusy}>
                Import local video
              </Button>
              <p className="text-left text-sm text-muted">
                Creates a local project, imports metadata, and opens the workspace when ready.
              </p>
              {importStatus && (
                <p className="text-left text-sm text-muted" role="status" aria-live="polite">
                  {importStatus}
                </p>
              )}
            </div>
            <YoutubeSecondaryAction />
            <section
              className="mt-8 flex w-full flex-col gap-3"
              aria-labelledby="recent-projects-story-heading"
            >
              <h2
                id="recent-projects-story-heading"
                className="mb-2 text-left text-sm font-semibold uppercase tracking-wider text-muted"
              >
                Recent Projects
              </h2>
              {list}
            </section>
          </div>
        </PageContent>
      </PageContainer>
    </Page>
  );
}

export function ProjectListRowsStory({ projects }: { projects: HomeProject[] }) {
  return (
    <ul className="flex flex-col gap-2">
      {projects.map((project) => (
        <li key={project.title}>
          <button
            type="button"
            className="w-full rounded-lg border border-border bg-surface p-3 text-left transition-colors hover:bg-secondary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg"
          >
            <span className="block truncate text-sm font-semibold text-text">{project.title}</span>
            <span className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted">
              <span>{project.status}</span>
              <span>{project.sourceLabel}</span>
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}

function YoutubeSecondaryAction() {
  return (
    <>
      <div className="flex w-full items-center gap-4">
        <hr className="flex-1 border-muted" />
        <span className="text-sm font-medium uppercase tracking-widest text-muted">
          or add a YouTube source
        </span>
        <hr className="flex-1 border-muted" />
      </div>
      <form className="flex flex-col gap-3" aria-label="Create YouTube project">
        <Input label="YouTube URL" placeholder="https://youtube.com/watch?v=..." />
        <Button type="submit" variant="secondary">
          Add from YouTube
        </Button>
      </form>
    </>
  );
}
