import { ProjectHeader } from '../../../widgets/project-header';
import { Page } from '../../../shared/ui/page-layout';
import { SourceWorkspace } from './SourceWorkspace';
import { useNavigation } from '@/shared/router';
import { SubtitleWorkspace } from './SubtitleWorkspace';

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
        {pipelineStep === 'source' ? (
          <SourceWorkspace />
        ) : (
          <SubtitleWorkspace />
        )}
      </section>
    </Page>
  );
};
