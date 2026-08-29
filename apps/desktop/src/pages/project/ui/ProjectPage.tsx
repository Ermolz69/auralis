import { lazy, Suspense } from 'react';
import { ProjectHeader } from '../../../widgets/project-header';
import { Page } from '../../../shared/ui/page-layout';
import { useNavigation } from '@/shared/router';

const SourceWorkspace = lazy(() =>
  import('./SourceWorkspace').then((module) => ({ default: module.SourceWorkspace })),
);
const SubtitleWorkspace = lazy(() =>
  import('./SubtitleWorkspace').then((module) => ({ default: module.SubtitleWorkspace })),
);

export const ProjectPage = () => {
  const { pipelineStep } = useNavigation();

  return (
    <Page className="flex h-full min-h-0 flex-col overflow-hidden">
      <ProjectHeader />
      <section
        className="flex-1 min-h-0 overflow-hidden"
        data-testid="project-workspace"
        aria-label="Project workspace"
      >
        <Suspense fallback={<WorkspaceLoading />}>
          {pipelineStep === 'source' ? <SourceWorkspace /> : <SubtitleWorkspace />}
        </Suspense>
      </section>
    </Page>
  );
};

function WorkspaceLoading() {
  return (
    <div className="flex h-full items-center justify-center text-xs text-muted" role="status">
      Loading workspace…
    </div>
  );
}
