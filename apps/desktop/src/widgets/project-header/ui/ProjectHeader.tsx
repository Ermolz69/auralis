import { RunDubbing } from '../../../features/run-dubbing';
import {
  PageHeader,
  PageHeaderGroup,
  PageTitle,
  PageDescription,
  PageActions,
} from '../../../shared/ui/page-layout';
import { useProjectContext } from '@/entities/project';
import { MediaSummary } from './MediaSummary';

export const ProjectHeader = () => {
  const { project } = useProjectContext();

  return (
    <PageHeader className="px-6 py-4 bg-surface border-b border-muted items-center">
      <PageHeaderGroup>
        <div className="flex items-center gap-3">
          <PageTitle className="!text-xl">{project?.title || 'Loading Project...'}</PageTitle>
          {project?.status && (
            <span className="px-2 py-0.5 rounded text-xs font-medium bg-secondary text-secondary-foreground">
              {project.status.toUpperCase()}
            </span>
          )}
        </div>
        {project?.metadata ? (
          <MediaSummary metadata={project.metadata} />
        ) : (
          <PageDescription className="!text-sm mt-1">
            {project?.source?.kind === 'managedLocalFile'
              ? project.source.originalFilename
              : project?.source?.kind === 'externalLocalFile'
                ? project.source.path
                : project?.source?.kind === 'youtubeUrl' || project?.source?.kind === 'remoteUrl'
                  ? project.source.url
                  : 'No media source attached'}
          </PageDescription>
        )}
      </PageHeaderGroup>
      <PageActions>
        <RunDubbing />
      </PageActions>
    </PageHeader>
  );
};
