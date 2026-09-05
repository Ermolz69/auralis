import { lazy, Suspense } from 'react';
import { ProjectHeader } from '../../../widgets/project-header';
import { Page } from '../../../shared/ui/page-layout';
import { StateView } from '../../../shared/ui/state-view';
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
    <StateView
      title="Loading workspace…"
      density="compact"
      loading
      role="status"
      live="polite"
      className="h-full"
    />
  );
}
