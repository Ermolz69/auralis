import { ProjectList } from '../../../features/project-list';
import { Page } from '../../../shared/ui/page-layout';

export const HomePage = () => {
  return (
    <Page className="flex h-full min-h-0 flex-col overflow-hidden">
      <header className="flex shrink-0 items-start border-b border-border px-5 py-5 sm:px-8">
        <div>
          <h1 className="text-xl font-semibold text-text">Projects</h1>
          <p className="mt-0.5 text-xs text-muted">Local media workspaces on this device</p>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4 sm:px-6 lg:px-8">
        <div className="mx-auto w-full max-w-5xl">
          <ProjectList />
        </div>
      </div>
    </Page>
  );
};
